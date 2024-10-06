use std::net::TcpListener;

use actix_web::{dev::Server, web::{self, Data}, App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use secrecy::SecretString;
use tracing_actix_web::TracingLogger;

use crate::{configuration::Settings, domain::user_email::UserEmail, email_client::EmailClient, jwt_auth::Tokenizer, routes::{authentication::{login::login, register::register}, confirm::confirm, health_check, inventory::{get_inventory, post_inventory}, order::{delete_order, get_order, post_order, update_order}, profile::{get_profile, post_profile}}};

#[derive(Clone)]
pub struct BaseUrl(pub String);

pub struct Application{
    pub host: String,
    pub port: u16,
    pub server: Server
}

impl Application {
    pub async fn new(settings: Settings) -> Result<Self, anyhow::Error>{
        let listener = TcpListener::bind(format!("{}:{}",
                settings.application.host,
                settings.application.port
        ))?;

        let port = listener.local_addr()?
                        .port();

        let manager = ConnectionManager::<PgConnection>::new(settings.database.get_database_table_url());
        let pool = Pool::new(manager)
                    .expect("Failed to create pool for application");


        let sender = UserEmail::parse(settings.email.sender).unwrap();
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

        let tokenizer = Tokenizer::new(&settings.jwt);

        let server = HttpServer::new(move || {
            App::new()
                .wrap(TracingLogger::default())
                .route("/health", web::get().to(health_check))
                .route("/register", web::post().to(register))
                .route("/confirm", web::get().to(confirm))
                .route("/login", web::post().to(login))
                .route("/inventory", web::get().to(get_inventory))
                .route("/order", web::get().to(get_order))
                .service(web::scope("/user")
                    .route("/profile", web::get().to(get_profile))
                    .route("/profile", web::post().to(post_profile))
                    .route("/order", web::post().to(post_order))
                    .route("/order", web::delete().to(delete_order))
                )
                .service(web::scope("/admin")
                    .route("/inventory", web::post().to(post_inventory))
                    .route("/order", web::put().to(update_order))
                    .route("/order", web::delete().to(delete_order))
                )
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(email_client.clone()))
                .app_data(Data::new(base_url.clone()))
                .app_data(Data::new(tokenizer.clone()))
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

