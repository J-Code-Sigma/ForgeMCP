CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS memory_chunks (
    id UUID PRIMARY KEY,
    content TEXT NOT NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    embedding vector(384) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON memory_chunks USING hnsw (embedding vector_cosine_ops);
