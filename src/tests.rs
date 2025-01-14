use super::*;

#[sqlx::test(fixtures("fixtures/prs.sql", "fixtures/reviews.sql"))]
async fn db_get_updated_at(pool: SqlitePool) -> Result<()> {
    let result = db::get_updated_at(&pool, "test-repo").await?;
    assert_eq!(result.len(), 2);
    assert!(result.iter().any(|(num, _)| *num == 1));
    assert!(result.iter().any(|(num, _)| *num == 2));

    // Test non-existent repo
    let result = db::get_updated_at(&pool, "non-existent-repo").await?;
    assert_eq!(result.len(), 0);

    Ok(())
}

#[sqlx::test(fixtures("fixtures/prs.sql", "fixtures/reviews.sql"))]
async fn summary_with_data(pool: SqlitePool) -> Result<()> {
    summary(&pool).await?;
    Ok(())
}

#[sqlx::test]
async fn summary_empty_db(pool: SqlitePool) -> Result<()> {
    summary(&pool).await?;
    Ok(())
}

#[sqlx::test(fixtures("fixtures/prs.sql"))]
async fn summary_no_reviews(pool: SqlitePool) -> Result<()> {
    summary(&pool).await?;
    Ok(())
}
