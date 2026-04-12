// Imports

use std::env;

pub struct Config {
    pub port:         u16,
    pub api_key:      String,
    pub database_url: String,
}

// Load configuration from environment.
// Fail if api key is not matching requirements
pub fn load() -> Config {
    dotenvy::dotenv().ok();

    let api_key = env::var("API_KEY")
        .expect("API_KEY must be set in the environment (no default allowed)");

    if api_key.len() < 32 {
        panic!("API_KEY must be at least 32 characters long");
    }

    Config {
        port: env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse()
            .expect("PORT must be a number"),

        api_key,

        database_url: env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set"),
    }
}