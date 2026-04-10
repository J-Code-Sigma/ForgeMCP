#![allow(dead_code)]
use anyhow::Result;
use pgvector::Vector;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

/// Provides connectivity to PostgreSQL and pgvector for semantic search.
pub struct DbClient {
    pool: PgPool,
}

impl DbClient {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await?;

        // Run embedded migrations dynamically on boot
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn save_to_memory(&self, content: &str, tags: Vec<String>, embedding: Vec<f32>) -> Result<()> {
        let id = Uuid::new_v4();
        let vector = Vector::from(embedding);

        // We use non-macro runtime query paths to avoid SQLX preventing offline cargo builds
        sqlx::query("INSERT INTO memory_chunks (id, content, tags, embedding) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(content)
            .bind(&tags)
            .bind(vector)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn search_memory(&self, query_embedding: Vec<f32>, limit: i64) -> Result<Vec<String>> {
        let vector = Vector::from(query_embedding);

        // Uses <=> operator for cosine distance sorting natively in pgvector
        let row_results: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM memory_chunks ORDER BY embedding <=> $1 LIMIT $2"
        )
            .bind(vector)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        // Map results
        Ok(row_results.into_iter().map(|rec| rec.0).collect())
    }
}
