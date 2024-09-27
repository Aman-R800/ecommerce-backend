use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, web::{self, Data}, App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use secrecy::SecretString;
use tracing_actix_web::TracingLogger;

use crate::{configuration::Settings, domain::subscriber_email::SubscriberEmail, email_client::EmailClient, routes::{authentication::register::register, confirm::confirm, health_check}};

#[derive(Clone)]
pub struct BaseUrl(pub String);

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


        let sender = SubscriberEmail::parse(settings.email.sender).unwrap();
        let key = SecretString::from(settings.email.key.to_string());

        let email_client = EmailClient::new(
            settings.email.api_uri,
            sender,
            key,
            3
        );


        let base_url = BaseUrl(format!(
            "http://{}:{}/",
            settings.application.host,
            settings.application.port
        ));
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
                .route("/confirm", web::get().to(confirm))
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(email_client.clone()))
                .app_data(Data::new(base_url.clone()))
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

