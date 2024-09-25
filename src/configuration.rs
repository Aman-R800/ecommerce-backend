use std::u16;

use config::{Config, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Settings{
    pub application: ApplicationSettings,
    pub database: DatabaseSettings
}

impl Settings{
    pub fn get() -> Self{
        let config = Config::builder()
            .add_source(File::with_name("configuration/base.yaml"))
            .add_source(File::with_name("configuration/local.yaml"))
            .build()
            .expect("Failed to get configuration")
            .try_deserialize::<Settings>()
            .expect("Failed to deserialize to Settings struct");

        config
    }
}

#[derive(Deserialize, Debug)]
pub struct ApplicationSettings{
    pub host: String,
    pub port: u16
}

#[derive(Deserialize, Debug)]
pub struct DatabaseSettings{
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String
}

impl DatabaseSettings{
    pub fn get_database_url(&self) -> String{
        format!("postgres://{}:{}@{}:{}",
            self.username,
            self.password,
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
