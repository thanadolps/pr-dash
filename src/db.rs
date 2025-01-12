use bon::builder;
use chrono::{DateTime, NaiveDateTime, Utc};
use color_eyre::{eyre::Ok, Result};
use octocrab::models::{pulls::PullRequest, pulls::Review};
use sqlx::{Executor, Sqlite, SqlitePool};

#[derive(Debug)]
pub struct PullRequestId {
    pub repo: String,
    pub number: i64,
}

pub async fn get_updated_at(
    db: impl Executor<'_, Database = Sqlite>,
    repo: &str,
) -> Result<Vec<(u64, DateTime<Utc>)>> {
    struct Result {
        id: i64,
        updated_at: NaiveDateTime,
    }

    let rows = sqlx::query_as!(
        Result,
        "SELECT id, updated_at FROM pull_request WHERE repo = $1",
        repo
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.id as u64, r.updated_at.and_utc()))
        .collect())
}

pub async fn upsert_pull_request(
    db: impl Executor<'_, Database = Sqlite>,
    repo: &str,
    pr: &PullRequest,
) -> Result<PullRequestId> {
    let id = pr.number as i64;
    let author = pr.user.as_ref().map(|u| u.login.as_str());
    let state = pr.state.as_ref().map(|s| format!("{:?}", s));
    let head = &pr.head.label;
    let base = &pr.base.label;
    let title = &pr.title;
    let body = pr.body.as_deref().unwrap_or("");
    let created_at = pr.created_at;
    let updated_at = pr.updated_at;

    sqlx::query!(
        r#"
        INSERT OR REPLACE INTO pull_request (id, repo, author, state, head, base, title, body, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        id,
        repo,
        author,
        state,
        head,
        base,
        title,
        body,
        created_at,
        updated_at
    )
    .execute(db)
    .await?;

    Ok(PullRequestId {
        repo: repo.to_string(),
        number: id,
    })
}

pub async fn upsert_review(
    db: impl Executor<'_, Database = Sqlite>,
    pr_id: &PullRequestId,
    review: &Review,
) -> Result<()> {
    let id = review.id.0 as i64;
    let author = review.user.as_ref().map(|u| u.login.as_str());
    let state = review.state.as_ref().map(|s| format!("{:?}", s));
    let submitted_at = review.submitted_at;

    sqlx::query!(
        r#"
        INSERT OR REPLACE INTO review (id, pr_repo, pr_id, author, state, submitted_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        id,
        pr_id.repo,
        pr_id.number,
        author,
        state,
        submitted_at
    )
    .execute(db)
    .await?;

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct SummaryClosePr {
    pub repo: String,
    pub author: String,
    pub count: u32,
}

#[builder]
pub async fn summary_closed_pr(
    #[builder(start_fn)] db: &SqlitePool,
    author: Option<String>,
    repo: Option<String>,
    base: Option<String>,
) -> Result<Vec<SummaryClosePr>> {
    let result = sqlx::query_as!(
        SummaryClosePr,
        r#"
        SELECT repo, author, count(*) as "count!: u32"
        FROM pull_request
        WHERE state = 'Closed'
            AND ($1 IS NULL OR author = $1)
            AND ($2 IS NULL OR repo = $2)
            AND ($3 IS NULL OR base = $3)
        GROUP BY repo, author
        ORDER BY repo, count(*) DESC
        "#,
        author,
        repo,
        base
    )
    .fetch_all(db)
    .await?;

    Ok(result)
}

#[derive(Debug, sqlx::FromRow)]
pub struct SummaryApprovePr {
    pub repo: String,
    pub approver: String,
    pub count: u32,
}

#[builder]
pub async fn summary_approved_pr(
    #[builder(start_fn)] db: &SqlitePool,
    author: Option<String>,
    repo: Option<String>,
    base: Option<String>,
) -> Result<Vec<SummaryApprovePr>> {
    let result = sqlx::query_as!(
        SummaryApprovePr,
        r#"
        SELECT pr_repo as "repo", author as "approver", count(distinct concat(pr_repo, pr_id)) as "count!: u32"
        FROM review
        WHERE state = 'Approved'
            AND ($1 IS NULL OR author = $1)
            AND ($2 IS NULL OR repo = $2)
            AND ($3 IS NULL OR pr_repo = $3)
        GROUP BY pr_repo, author
        ORDER BY pr_repo, count(*) DESC
        "#,
        author,
        repo,
        base
    )
    .fetch_all(db)
    .await?;

    Ok(result)
}
