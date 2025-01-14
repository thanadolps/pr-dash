INSERT INTO pull_request 
(id, repo, title, author, state, head, base, body, created_at, updated_at)
VALUES 
(1, 'test-repo', 'Merged PR', 'author1', 'Closed', 'head1', 'main', '', '2025-01-01T00:00:00Z', '2025-01-02T00:00:00Z'),
(2, 'test-repo', 'Open PR', 'author2', 'Open', 'head2', 'main', '', '2025-01-01T00:00:00Z', '2025-01-02T00:00:00Z');
