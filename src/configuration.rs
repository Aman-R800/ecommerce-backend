use std::u16;

use config::{Config, Environment, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

// Struct to store application config and settings
#[derive(Deserialize, Debug)]
pub struct Settings{
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email: EmailSettings,
    pub jwt: JWTSettings
}

impl Settings{
    // Build Settings through configuration yamls and environment variables
    pub fn get() -> Self{
        let env_source = Environment::default()
                            .separator("__");

        let config = Config::builder()
            .add_source(File::with_name("configuration/base.yaml"))
            .add_source(File::with_name("configuration/local.yaml"))
            .add_source(env_source)
            .build()
            .expect("Failed to get configuration")
            .try_deserialize::<Settings>()
            .expect("Failed to deserialize to Settings struct");

        dbg!(&config.email.api_uri);

        config
    }
}

// Settings related to application
#[derive(Deserialize, Debug)]
pub struct ApplicationSettings{
    pub host: String,
    pub port: u16,
}

// Settings related to database
#[derive(Deserialize, Debug)]
pub struct DatabaseSettings{
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: SecretString
}

// Settings related to email sending service
#[derive(Deserialize, Debug)]
pub struct EmailSettings{
    pub api_uri: String,
    pub sender: String,
    pub key: String
}

// Settings related to JWT
#[derive(Deserialize, Debug)]
pub struct JWTSettings{
    pub secret: String,
    pub expiry_hours: u64
}

impl DatabaseSettings{
    // get database url
    pub fn get_database_url(&self) -> String{
        format!("postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
    }

    // get database url with table
    pub fn get_database_table_url(&self) -> String{
        let mut base = self.get_database_url();
        base.push_str(format!("/{}", self.name).as_str());
        base
    }
}
