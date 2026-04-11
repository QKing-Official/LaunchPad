// Imports

use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;

pub mod queries;

// Setup the database connection
pub async fn connect(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

// Migrate the db from the migration folder
pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}