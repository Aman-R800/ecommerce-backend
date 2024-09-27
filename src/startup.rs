use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, web::{self, Data}, App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use tracing_actix_web::TracingLogger;

use crate::{configuration::Settings, routes::{authentication::register::register, health_check}};

pub struct Application{
    pub host: String,
    pub port: u16,
    pub server: Server
}

impl Application {
    async fn get_redis_store(settings: &Settings) -> RedisSessionStore{
        RedisSessionStore::new(
            format!("redis://{}:{}",
            settings.redis.host,
            settings.redis.port
        ))
        .await
        .unwrap()
    }

    pub async fn new(settings: Settings) -> Result<Self, anyhow::Error>{
        let listener = TcpListener::bind(format!("{}:{}",
                settings.application.host,
                settings.application.port
        ))?;

        let port = listener.local_addr()?
                        .port();

        let secret_key = Key::from(settings.get_key());
        let redis_store = Application::get_redis_store(&settings).await;

        let manager = ConnectionManager::<PgConnection>::new(settings.database.get_database_table_url());
        let pool = Pool::new(manager)
                    .expect("Failed to create pool for application");

        let server = HttpServer::new(move || {
            App::new()
                .wrap(
                    SessionMiddleware::new(
                        redis_store.clone(),
                        secret_key.clone()
                    )
                )
                .wrap(TracingLogger::default())
                .route("/health", web::get().to(health_check))
                .route("/register", web::post().to(register))
                .app_data(Data::new(pool.clone()))
        })
        .listen(listener)?
        .run();

        Ok(Application{
            host: settings.application.host,
            port,
            server
        })
    }
}

