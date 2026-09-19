-- Page visit statistics for the landing site.
CREATE TABLE IF NOT EXISTS page_visits (
    id BIGSERIAL PRIMARY KEY,
    visited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip_hash TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '/',
    user_agent TEXT
);
CREATE INDEX IF NOT EXISTS page_visits_visited_at_idx ON page_visits (visited_at);
