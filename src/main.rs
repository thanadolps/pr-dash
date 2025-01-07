use color_eyre::{eyre::WrapErr, Result};
use octocrab::{params::State, OctocrabBuilder};
use serde::Deserialize;
use sqlx::SqlitePool;

mod db;

#[derive(Deserialize)]
struct Env {
    token: String,
    database_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;

    let env = envy::from_env::<Env>()?;

    let org = "smartertravel";
    let repos = [
        "datasource-converter",
        "data-platform",
        "smarter-mail",
        "partner-feed",
    ];

    let db = SqlitePool::connect(&env.database_url).await?;

    let oc = OctocrabBuilder::new().personal_token(env.token).build()?;
    for &repo in repos.iter() {
        let pull = oc.pulls(org, repo);

        for i in 0u32.. {
            let page = pull
                .list()
                .state(State::Closed)
                .base("main")
                .per_page(100)
                .page(i)
                .send()
                .await?;

            if page.items.is_empty() {
                break;
            }

            println!(
                "page {} of {} has {} merged pull requests",
                i,
                repo,
                page.items.len()
            );

            for pr in page {
                let pr_id =
                    match db::insert_pull_request(&db, repo, &pr)
                        .await
                        .wrap_err_with(|| {
                            format!("failed to insert pull request {}/{}", repo, pr.number)
                        }) {
                        Ok(id) => id,
                        Err(e) => {
                            println!("skipping {}/{}: {:?}", repo, pr.number, e);
                            continue;
                        }
                    };

                assert_eq!(pr.number as i64, pr_id.number);

                let reviews = pull.list_reviews(pr.number).per_page(100).send().await?;
                assert_ne!(reviews.incomplete_results, Some(true));

                println!(
                    "[{:?}] ({:?} -> {:?})  - {:?}",
                    pr.user
                        .as_ref()
                        .map(|u| &u.login)
                        .unwrap_or(&"unknown".to_string()),
                    pr.head.label,
                    pr.base.label,
                    pr.title,
                );

                for review in reviews.items {
                    if let Err(e) = db::insert_review(&db, &pr_id, &review).await {
                        println!("failed to insert review: {}", e);
                        continue;
                    }
                    println!(
                        "\t{:?} by {:?} at {:?}",
                        review.state,
                        review
                            .user
                            .as_ref()
                            .map(|u| &u.login)
                            .unwrap_or(&"unknown".to_string()),
                        review.submitted_at
                    );
                }
            }
        }
    }
    Ok(())
}
