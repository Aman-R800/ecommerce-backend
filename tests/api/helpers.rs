use std::{error::Error, net::TcpListener};

use diesel::{pg::Pg, r2d2::ConnectionManager, Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ecommerce::{configuration::{DatabaseSettings, Settings}, startup::Application, telemetry::{get_subscriber, init_subscriber}, utils::DbPool};
use once_cell::sync::Lazy;
use r2d2::Pool;
use reqwest::redirect::Policy;
use uuid::Uuid;
use wiremock::MockServer;

static LOGGER_INSTANCE: Lazy<()> = Lazy::new(|| {
    let log_level = "info".to_string();
    let name = "ecommerce-test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(name, log_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(name, log_level, std::io::sink);
        init_subscriber(subscriber);
    }

    ()
});

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn run_migrations(connection: &mut impl MigrationHarness<Pg>) 
    -> Result<(), Box<dyn Error + Send + Sync + 'static>> 
{
    connection.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

pub struct TestApp{
    pub host: String,
    pub port: u16,
    pub pool: DbPool,
    pub email_api: MockServer,
    pub api_client: reqwest::Client
}

impl TestApp {
    fn create_db(settings: &DatabaseSettings) -> DbPool{
        let mut connection = PgConnection::establish(&settings.get_database_url())
                                .expect("Failed to connect to postgres database");

        let query = format!(r#"CREATE DATABASE "{}";"#, settings.name);
        diesel::sql_query(query)
            .execute(&mut connection)
            .expect("Failed to create test database");

        let pool = Pool::new(ConnectionManager::<PgConnection>::new(settings.get_database_table_url()))
            .expect("Failed to build connection pool to test database");

        let mut conn = pool.get().expect("Failed to get connection to test database");
        run_migrations(&mut conn).expect("Failed to run migrations");

        pool
    }

    pub fn get_app_url(&self) -> String{
        format!("http://{}:{}", self.host, self.port)
    }

    pub async fn spawn_app() -> TestApp{
        Lazy::force(&LOGGER_INSTANCE);

        let email_api = MockServer::start().await;

        let mut settings = Settings::get();
        settings.application.port = 0;
        settings.database.name = Uuid::new_v4().to_string();
        settings.email.api_uri = email_api.uri();

        let pool = TestApp::create_db(&settings.database);

        
        let application = Application::new(settings)
                            .await
                            .expect("Failed to build application");


        tokio::task::spawn(application.server);

        let api_client = reqwest::Client::builder()
                            .redirect(Policy::none())
                            .cookie_store(true)
                            .build()
                            .unwrap();

        return TestApp{
            host: application.host,
            port: application.port,
            pool,
            email_api,
            api_client
        }
    }

    pub fn get_confirmation_link(&self, text: &str) -> String{
        let links: Vec<_> = linkify::LinkFinder::new()
                    .links(text)
                    .filter(|l| *l.kind() == linkify::LinkKind::Url)
                    .collect();
        assert_eq!(links.len(), 1);
        let raw_link = links[0].as_str().to_owned();
        let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

        assert_eq!(confirmation_link.host_str().unwrap(), "localhost");
        confirmation_link.set_port(Some(self.port)).unwrap();

        confirmation_link.to_string()
    }

}
