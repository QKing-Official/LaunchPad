// Imports

use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::docker::client::DockerClient;
use crate::server::ports::PortAllocator;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppRecord {
    pub id:           Uuid,
    pub name:         String,
    pub image:        String,
    pub status:       String,
    pub container_id: Option<String>,
    pub memory_mb:    Option<i32>,
    pub cpu_shares:   Option<i32>,
    pub cpu_quota:    Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortMappingRecord {
    pub id:            Uuid,
    pub app_id:        Uuid,
    pub internal_port: i32,
    pub external_port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResponse {
    pub id:            Uuid,
    pub name:          String,
    pub image:         String,
    pub status:        String,
    pub container_id:  Option<String>,
    pub external_port: Option<i32>,
    pub internal_port: Option<i32>,
    pub memory_mb:     Option<i32>,
    pub cpu_shares:    Option<i32>,
}

#[derive(Clone)]
pub struct AppState {
    pub docker: Arc<DockerClient>,
    pub db:     PgPool,
    pub ports:  Arc<PortAllocator>,
}


// State of the client and port allocator
impl AppState {
    pub fn new(docker: DockerClient, db: PgPool) -> Self {
        Self {
            docker: Arc::new(docker),
            db,
            ports: Arc::new(PortAllocator::new(30000)),
        }
    }
}