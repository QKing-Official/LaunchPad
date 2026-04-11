// Imports

use sqlx::PgPool;
use uuid::Uuid;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::server::state::{AppRecord, PortMappingRecord};

// App related queries:

// Insert app
pub async fn insert_app(pool: &PgPool, app: &AppRecord) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO apps (id, name, image, status, container_id, memory_mb, cpu_shares, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
           ON CONFLICT (id) DO UPDATE
             SET status=EXCLUDED.status, container_id=EXCLUDED.container_id, updated_at=now()"#,
    )
    .bind(app.id).bind(&app.name).bind(&app.image).bind(&app.status)
    .bind(&app.container_id).bind(app.memory_mb).bind(app.cpu_shares)
    .execute(pool).await?;
    Ok(())
}

// App status
pub async fn update_app_status(pool: &PgPool, id: Uuid, status: &str, container_id: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE apps SET status=$1, container_id=$2, updated_at=now() WHERE id=$3")
        .bind(status).bind(container_id).bind(id)
        .execute(pool).await?;
    Ok(())
}

// List apps
pub async fn list_apps(pool: &PgPool) -> Result<Vec<AppRecord>> {
    let rows = sqlx::query_as::<_, AppRecord>(
        "SELECT id, name, image, status, container_id, memory_mb, cpu_shares, cpu_quota FROM apps ORDER BY created_at DESC",
    ).fetch_all(pool).await?;
    Ok(rows)
}

// Get app information
pub async fn get_app(pool: &PgPool, id: Uuid) -> Result<Option<AppRecord>> {
    let row = sqlx::query_as::<_, AppRecord>(
        "SELECT id, name, image, status, container_id, memory_mb, cpu_shares, cpu_quota FROM apps WHERE id=$1",
    ).bind(id).fetch_optional(pool).await?;
    Ok(row)
}

// Delete the app
pub async fn delete_app(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM apps WHERE id=$1").bind(id).execute(pool).await?;
    Ok(())
}

// Port mapping

// Insert port mapping
pub async fn insert_port_mapping(pool: &PgPool, m: &PortMappingRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO port_mappings (id, app_id, internal_port, external_port) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
    ).bind(m.id).bind(m.app_id).bind(m.internal_port).bind(m.external_port)
    .execute(pool).await?;
    Ok(())
}

// Get the existing port mappings of the container
pub async fn get_port_mappings(pool: &PgPool, app_id: Uuid) -> Result<Vec<PortMappingRecord>> {
    let rows = sqlx::query_as::<_, PortMappingRecord>(
        "SELECT id, app_id, internal_port, external_port FROM port_mappings WHERE app_id=$1",
    ).bind(app_id).fetch_all(pool).await?;
    Ok(rows)
}

// Delete the port mappings as well
pub async fn delete_port_mappings(pool: &PgPool, app_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM port_mappings WHERE app_id=$1").bind(app_id).execute(pool).await?;
    Ok(())
}

// Webhooks

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookRecord {
    pub id:     Uuid,
    pub app_id: Uuid,
    pub url:    String,
}
// List the existing webhooks
pub async fn list_webhooks(pool: &PgPool, app_id: Uuid) -> Result<Vec<WebhookRecord>> {
    let rows = sqlx::query_as::<_, WebhookRecord>(
        "SELECT id, app_id, url FROM webhooks WHERE app_id=$1",
    ).bind(app_id).fetch_all(pool).await?;
    Ok(rows)
}

// Insert a webhook
pub async fn insert_webhook(pool: &PgPool, id: Uuid, app_id: Uuid, url: &str) -> Result<()> {
    sqlx::query("INSERT INTO webhooks (id, app_id, url) VALUES ($1,$2,$3)")
        .bind(id).bind(app_id).bind(url).execute(pool).await?;
    Ok(())
}

pub async fn delete_webhook(pool: &PgPool, id: Uuid, app_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM webhooks WHERE id=$1 AND app_id=$2")
        .bind(id).bind(app_id).execute(pool).await?;
    Ok(())
}

// App based tokens for safety
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenRecord {
    pub id:     Uuid,
    pub app_id: Uuid,
    pub label:  String,
}

// List the app based tokens
pub async fn list_tokens(pool: &PgPool, app_id: Uuid) -> Result<Vec<TokenRecord>> {
    let rows = sqlx::query_as::<_, TokenRecord>(
        "SELECT id, app_id, label FROM app_tokens WHERE app_id=$1",
    ).bind(app_id).fetch_all(pool).await?;
    Ok(rows)
}

// Insert app based tokens
pub async fn insert_token(pool: &PgPool, id: Uuid, app_id: Uuid, token: &str, label: &str) -> Result<()> {
    sqlx::query("INSERT INTO app_tokens (id, app_id, token, label) VALUES ($1,$2,$3,$4)")
        .bind(id).bind(app_id).bind(token).bind(label).execute(pool).await?;
    Ok(())
}

// Delete an app based token for safety
pub async fn delete_token(pool: &PgPool, id: Uuid, app_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM app_tokens WHERE id=$1 AND app_id=$2")
        .bind(id).bind(app_id).execute(pool).await?;
    Ok(())
}

// Validate a token to make sure all acces is authorised
pub async fn validate_token(pool: &PgPool, token: &str, app_id: Uuid) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM app_tokens WHERE token=$1 AND app_id=$2",
    ).bind(token).bind(app_id).fetch_optional(pool).await?;
    Ok(row.is_some())
}