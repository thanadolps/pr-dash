use color_eyre::Result;
use octocrab::models::{pulls::PullRequest, pulls::Review};
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct PullRequestId {
    pub repo: String,
    pub number: i64,
}

pub async fn insert_pull_request(
    db: &SqlitePool,
    repo: &str,
    pr: &PullRequest,
) -> Result<PullRequestId> {
    let id = pr.number as i64;
    let author = pr
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("unknown");
    let state = pr
        .state
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "unknown".to_string());
    let head = &pr.head.label;
    let base = &pr.base.label;
    let title = &pr.title;
    let body = pr.body.as_deref().unwrap_or("");

    let result = sqlx::query!(
        r#"
        INSERT INTO pull_request (id, repo, author, state, head, base, title, body)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        id,
        repo,
        author,
        state,
        head,
        base,
        title,
        body
    )
    .execute(db)
    .await?;

    Ok(PullRequestId {
        repo: repo.to_string(),
        number: id,
    })
}

pub async fn insert_review(db: &SqlitePool, pr_id: &PullRequestId, review: &Review) -> Result<()> {
    let id = review.id.0 as i64;
    let author = review
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("unknown");
    let state = review
        .state
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "unknown".to_string());
    let submitted_at = review.submitted_at.map(|dt| dt.timestamp()).unwrap_or(0);

    sqlx::query!(
        r#"
        INSERT INTO review (id, pr_repo, pr_id, author, state, submitted_at)
        VALUES (?, ?, ?, ?, ?, ?)
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
