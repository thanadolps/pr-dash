-- Add up migration script here
CREATE TABLE IF NOT EXISTS review (
    id INTEGER NOT NULL PRIMARY KEY,
    pr_repo TEXT NOT NULL,
    pr_id INTEGER NOT NULL,
    author TEXT NOT NULL,
    state TEXT NOT NULL,
    submitted_at DATETIME NOT NULL,
    FOREIGN KEY (pr_repo, pr_id) REFERENCES pull_request(repo, id)
)