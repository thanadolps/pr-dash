use std::collections::BTreeMap;

use color_eyre::{eyre::WrapErr, Result, Section};
use dialoguer::Select;
use octocrab::{params::State, Octocrab, OctocrabBuilder};
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

    let env = envy::from_env::<Env>().wrap_err("failed parse envar")?;
    let db = SqlitePool::connect(&env.database_url).await?;
    let oc = OctocrabBuilder::new().personal_token(env.token).build()?;

    match Select::new()
        .items(&["update", "summary"])
        .default(0)
        .interact()?
    {
        0 => update_pull_requests(&db, &oc).await?,
        1 => todo!(),
        _ => unreachable!(),
    }

    Ok(())
}

async fn update_pull_requests(db: &SqlitePool, oc: &Octocrab) -> Result<()> {
    let org = "smartertravel";
    let repos = [
        "datasource-converter",
        "data-platform",
        "smarter-mail",
        "partner-feed",
    ];

    for &repo in repos.iter() {
        let pull = oc.pulls(org, repo);

        // get all PRs
        let mut prs = Vec::new();
        for i in 0u32.. {
            let page = pull
                .list()
                .state(State::All)
                .per_page(100)
                .page(i)
                .send()
                .await?;
            println!(
                "page {} of {} has {} merged pull requests",
                i,
                repo,
                page.items.len()
            );

            if page.items.is_empty() {
                break;
            }
            prs.extend(page.items);
        }
        println!("Collected {} PRs", prs.len());

        // determine PR that need to be updated
        let cached_updated_at = db::get_updated_at(db, repo)
            .await?
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let need_update = prs
            .iter()
            .filter(|pr| {
                cached_updated_at
                    .get(&pr.number)
                    .map_or(true, |updated_at| updated_at < &pr.updated_at.unwrap())
            })
            .collect::<Vec<_>>();
        println!("{} PRs need to be updated", need_update.len());

        for pr in need_update {
            // get the reviews for PR
            // (surely there is no more than 100 reviews per PR)
            let reviews = pull.list_reviews(pr.number).per_page(100).send().await?;
            assert_ne!(reviews.incomplete_results, Some(true));

            println!(
                "[{:?}] ({:?} -> {:?})  - {:?}",
                pr.user.as_ref().map(|u| &u.login),
                pr.head.label,
                pr.base.label,
                pr.title,
            );

            let mut tx = db.begin().await?;

            let pr_id = match db::upsert_pull_request(&mut *tx, repo, &pr)
                .await
                .wrap_err_with(|| format!("failed to upsert pull request {}/{}", repo, pr.number))
            {
                Ok(id) => id,
                Err(e) => {
                    println!("skipping {}/{}: {:?}", repo, pr.number, e.note(repo));
                    continue;
                }
            };
            assert_eq!(pr.number as i64, pr_id.number);

            for review in reviews.items {
                if let Err(e) = db::upsert_review(&mut *tx, &pr_id, &review).await {
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

            tx.commit().await?;
        }
    }

    Ok(())
}
