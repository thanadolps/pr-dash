-- Add up migration script here
CREATE TABLE IF NOT EXISTS pull_request (
    id INTEGER NOT NULL,
    repo TEXT NOT NULL,
    author TEXT NOT NULL,
    state TEXT NOT NULL,
    head TEXT NOT NULL,
    base TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    PRIMARY KEY (repo, id)
)