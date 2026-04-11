// Imports

use std::env;

pub struct Config {
    pub port:         u16,
    pub api_key:      String,
    pub database_url: String,
}


// Load the information from the .env
// This is currently just the API key and the port
// If one of those are changed they are changes across all areas of the daemon
// So please be careful!
pub fn load() -> Config {
    dotenvy::dotenv().ok();

    Config {
        port: env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse()
            .expect("PORT must be a number"),

        api_key: env::var("API_KEY")
            .unwrap_or_else(|_| "supersecret123".into()),

        database_url: env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set"),
    }
}