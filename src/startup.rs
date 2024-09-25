use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::{configuration::ApplicationSettings, routes::health_check};

pub struct Application{
    pub host: String,
    pub port: u16
}

impl Application {
    pub fn new(settings: ApplicationSettings) -> Self{
        Application{
            host: settings.host,
            port: settings.port
        }
    }

    pub fn get_server(&mut self) -> Result<Server, anyhow::Error>{
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))?;

        self.port = listener.local_addr()?
                        .port();

        let server = HttpServer::new(|| {
            App::new()
                .wrap(TracingLogger::default())
                .route("/health", web::get().to(health_check))
        })
        .listen(listener)?
        .run();

        Ok(server)
    }
}

