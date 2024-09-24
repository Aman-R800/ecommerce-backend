use actix_web::{dev::Server, web, App, HttpServer};

use crate::{configuration::Settings, routes::health_check};

pub struct Application{
    pub host: String,
    pub port: u16
}

impl Application {
    pub fn new(settings: Settings) -> Self{
        Application{
            host: settings.host,
            port: settings.port
        }
    }

    pub fn get_server(&self) -> Result<Server, anyhow::Error>{
        let server = HttpServer::new(|| {
            App::new()
                .route("/health", web::get().to(health_check))
        })
        .bind((self.host.as_str(), self.port))?
        .run();

        Ok(server)
    }
}

