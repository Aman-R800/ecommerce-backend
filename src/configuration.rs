use std::u16;

use config::{Config, Environment, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Settings{
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub email: EmailSettings,
    pub jwt: JWTSettings
}

impl Settings{
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

    pub fn get_key(&self) -> &[u8] {
        self.redis.key.as_bytes() 
    }
}

#[derive(Deserialize, Debug)]
pub struct ApplicationSettings{
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings{
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: SecretString
}

#[derive(Deserialize, Debug)]
pub struct RedisSettings{
    pub host: String,
    pub port: u16,
    pub key: String
}

#[derive(Deserialize, Debug)]
pub struct EmailSettings{
    pub api_uri: String,
    pub sender: String,
    pub key: String
}

#[derive(Deserialize, Debug)]
pub struct JWTSettings{
    pub secret: String,
    pub expiry_hours: u64
}

impl DatabaseSettings{
    pub fn get_database_url(&self) -> String{
        format!("postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
    }

    pub fn get_database_table_url(&self) -> String{
        let mut base = self.get_database_url();
        base.push_str(format!("/{}", self.name).as_str());
        base
    }
}
