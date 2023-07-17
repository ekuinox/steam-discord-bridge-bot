use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, PgPool};

#[derive(Debug)]
pub struct DbClient {
    pool: PgPool,
}

#[derive(Deserialize, Serialize, FromRow, Debug)]
pub struct UserRow {
    pub discord_id: String,
    pub steam_id: String,
}

impl DbClient {
    pub async fn new(pool: PgPool) -> Result<DbClient> {
        pool.execute(include_str!("../schema.sql"))
            .await
            .context("execute schema.sql failure")?;
        Ok(DbClient { pool })
    }

    pub async fn get_user(&self, discord_id: &str) -> Result<UserRow> {
        let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(user)
    }

    pub async fn insert_user(&self, discord_id: &str, steam_id: &str) -> Result<UserRow> {
        let user: UserRow = sqlx::query_as("INSERT INTO users (discord_id, steam_id) VALUES ($1, $2) RETURNING discord_id, steam_id").bind(discord_id).bind(steam_id).fetch_one(&self.pool).await?;
        Ok(user)
    }

    pub async fn update_user(&self, discord_id: &str, steam_id: &str) -> Result<UserRow> {
        let user: UserRow = sqlx::query_as(
            "UPDATE users SET steam_id = $1 WHERE discord_id = $2 RETURNING discord_id, steam_id",
        )
        .bind(steam_id)
        .bind(discord_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }
}
