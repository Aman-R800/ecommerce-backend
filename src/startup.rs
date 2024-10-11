use std::net::TcpListener;

use actix_web::{dev::Server, web::{self, Data}, App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use secrecy::SecretString;
use tracing_actix_web::TracingLogger;

use crate::{auth::jwt::Tokenizer, configuration::Settings, domain::user_email::UserEmail, email_client::EmailClient, routes::{authentication::{login::login, register::register}, confirm::confirm, health_check, inventory::{get_inventory, post_inventory}, order::{delete_order, get_order, post_order, update_order}, profile::{get_profile, post_profile}}};

// Base URL of application
#[derive(Clone)]
pub struct BaseUrl(pub String);

// Application related data and server
pub struct Application{
    pub host: String,
    pub port: u16,
    pub server: Server
}

impl Application {
    // Create new application from server
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
                .route("/health", web::get().to(health_check)) // Route to check if api is running
                .route("/register", web::post().to(register)) // Route for user to register
                .route("/confirm", web::get().to(confirm)) // Confirmation endpoint for user
                .route("/login", web::post().to(login)) // Route for user to login
                .route("/inventory", web::get().to(get_inventory)) // Route to view items available
                .route("/order", web::get().to(get_order)) // Route to view order details
                .service(web::scope("/user")
                    .route("/profile", web::get().to(get_profile)) // Route to view user profile
                                                                   // details

                    .route("/profile", web::post().to(post_profile)) // Route to post user profile
                                                                     // details

                    .route("/order", web::post().to(post_order)) // Route to create an order
                    .route("/order", web::delete().to(delete_order)) // Route to delete an order
                )
                .service(web::scope("/admin")
                    .route("/inventory", web::post().to(post_inventory)) // Route to post items to
                                                                         // inventory

                    .route("/order", web::put().to(update_order)) // Route to update order status
                    .route("/order", web::delete().to(delete_order)) // Route to delete an order
                )
                .app_data(Data::new(pool.clone())) // Database Connection Pool
                .app_data(Data::new(email_client.clone())) // Email Client
                .app_data(Data::new(base_url.clone())) // Base URL
                .app_data(Data::new(tokenizer.clone())) // JWT encoder and decoder
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

