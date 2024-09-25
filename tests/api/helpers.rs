use std::error::Error;

use diesel::{pg::Pg, r2d2::ConnectionManager, Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ecommerce::{configuration::{DatabaseSettings, Settings}, startup::Application, utils::DbPool};
use r2d2::Pool;
use uuid::Uuid;

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
    pub pool: DbPool
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
        let mut settings = Settings::get();
        settings.application.port = 0;
        settings.database.name = Uuid::new_v4().to_string();

        let pool = TestApp::create_db(&settings.database);

        
        let mut application = Application::new(settings.application);
        let server = application.get_server().expect("Failed to get server");

        tokio::task::spawn(server);

        return TestApp{
            host: application.host,
            port: application.port,
            pool 
        }
    }
}
