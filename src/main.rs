use std::collections::BTreeMap;

use color_eyre::{eyre::WrapErr, Result};
use dialoguer::Select;
use octocrab::{
    models::pulls::PullRequest,
    params::{pulls::Sort, Direction, State},
    Octocrab, OctocrabBuilder,
};
use serde::Deserialize;
use sqlx::SqlitePool;

mod db;
#[cfg(test)]
mod tests;

#[derive(Deserialize)]
struct Env {
    token: String,
    database_url: String,
    org: String,
    repos: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    color_eyre::install()?;

    let env = envy::from_env::<Env>().wrap_err("failed parse envar")?;
    let db = SqlitePool::connect(&env.database_url).await?;
    let token = env.token.clone();
    let oc = OctocrabBuilder::new().personal_token(token).build()?;

    match Select::new()
        .items(&["update", "summary"])
        .default(0)
        .interact()?
    {
        0 => update_pull_requests(&db, &oc, &env).await?,
        1 => summary(&db).await?,
        _ => unreachable!(),
    }

    Ok(())
}

async fn update_pull_requests(db: &SqlitePool, oc: &Octocrab, env: &Env) -> Result<()> {
    for repo in env.repos.iter() {
        let pull = oc.pulls(&env.org, repo);

        // get info on cached PR
        let cached_updated_at = db::get_updated_at(db, repo)
            .await?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        // fetch the PRs
        let mut prs = Vec::new();
        for i in 0u32.. {
            let page = pull
                .list()
                .state(State::All)
                .per_page(100)
                .sort(Sort::Updated)
                .direction(Direction::Descending)
                .page(i)
                .send()
                .await?;
            println!(
                "page {} of {} has {} merged pull requests",
                i,
                repo,
                page.items.len()
            );
            prs.extend_from_slice(&page.items);

            // stopping condition
            match (page.items.last(), cached_updated_at.values().max()) {
                // no more PRs
                (None, _) => {
                    println!("stop fetching, no more PRs");
                    break;
                }
                // still more PRs but cached PRs will always be up-to-date or newer from this point
                // (only work because PRs are sorted by updated_at desc)
                (Some(PullRequest { updated_at, .. }), Some(cached))
                    if *cached >= updated_at.unwrap() =>
                {
                    println!(
                        "stop fetching, cached PRs are up to date ({} >= {})",
                        cached,
                        updated_at.unwrap()
                    );
                    break;
                }
                // still more PRs
                _ => {}
            }
        }
        println!("Collected {} PRs", prs.len());

        // determine PR that need to be further updated
        let mut need_update = prs
            .iter()
            .filter(|pr| {
                cached_updated_at
                    .get(&pr.number)
                    .map_or(true, |updated_at| updated_at < &pr.updated_at.unwrap())
            })
            .collect::<Vec<_>>();
        // sort by updated_at in ascending order - older PRs come first.
        // if the process gets interrupted, a rerun will continue from where we left off since we process PRs from oldest to newest
        need_update.sort_by_key(|pr| pr.updated_at);
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

            let pr_id = db::upsert_pull_request(&mut *tx, repo, pr)
                .await
                .wrap_err_with(|| {
                    format!("failed to upsert pull request {}/{}", repo, pr.number)
                })?;
            assert_eq!(pr.number as i64, pr_id.number);

            for review in reviews.items {
                db::upsert_review(&mut *tx, &pr_id, &review)
                    .await
                    .wrap_err_with(|| {
                        format!(
                            "failed to upsert review {}/{}/{} ({})",
                            repo,
                            pr.number,
                            review.id,
                            review.body_text.unwrap_or_default()
                        )
                    })?;
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

async fn summary(db: &SqlitePool) -> Result<()> {
    let summary = db::summary_closed_pr(db).call().await?;
    let mut table = comfy_table::Table::new();
    table.set_header(["repo", "author", "count"]);
    for s in summary {
        table.add_row([s.repo, s.author, s.count.to_string()]);
    }
    println!("Closed PR\n{table}");

    let summary = db::summary_approved_pr(db).call().await?;
    let mut table = comfy_table::Table::new();
    table.set_header(["repo", "approver", "count"]);
    for s in summary {
        table.add_row([s.repo, s.approver, s.count.to_string()]);
    }
    println!("Approved PR\n{table}");

    Ok(())
}
