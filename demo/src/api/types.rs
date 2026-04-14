// Imports

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl ApiError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiOk {
    pub message: String,
}

// Just something to see if the api is working
impl ApiOk {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}