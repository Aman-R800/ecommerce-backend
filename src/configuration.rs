use std::u16;

use config::{Config, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Settings{
    pub host: String,
    pub port: u16
}

impl Settings{
    pub fn get() -> Self{
        let config = Config::builder()
            .add_source(File::with_name("configuration/base.yaml"))
            .build()
            .expect("Failed to get configuration")
            .try_deserialize::<Settings>()
            .expect("Failed to deserialize to Settings struct");

        config
    }
}
