use std::error::Error;

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use diesel::{pg::Pg, r2d2::ConnectionManager, Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ecommerce::{configuration::{DatabaseSettings, Settings}, models::User, startup::Application, telemetry::{get_subscriber, init_subscriber}, utils::DbPool};
use fake::{faker::internet::en::FreeEmail, Fake};
use once_cell::sync::Lazy;
use r2d2::Pool;
use rand::rngs::OsRng;
use reqwest::redirect::Policy;
use serde::Serialize;
use uuid::Uuid;
use wiremock::{matchers::{header_exists, path}, Mock, MockServer, ResponseTemplate};

use crate::registration::ReceiveEmailRequest;

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

pub struct TestUser{
    pub user_id: Uuid,
    pub email: String,
    pub password: String
}

impl TestUser {
    fn generate(admin: bool, pool: &DbPool) -> TestUser{
        use ecommerce::schema::users;

        let mut conn = pool.get().unwrap();
        
        let salt = SaltString::generate(&mut OsRng);
        let password_phc = Argon2::default()
                            .hash_password(&"testpassword".as_bytes(), &salt)
                            .unwrap()
                            .to_string(); 

        let user = User{
            user_id: Uuid::new_v4(),
            email: FreeEmail().fake(),
            name: "test name".to_string(),
            password: password_phc,
            is_admin: admin,
            status: Some("confirmed".to_string())
        };

        diesel::insert_into(
            users::table
        )
        .values(&user)
        .execute(&mut conn)
        .unwrap();

        TestUser{
            user_id: user.user_id,
            email: user.email,
            password: "testpassword".to_string()
        }
    }
}

pub struct TestApp{
    pub host: String,
    pub port: u16,
    pub pool: DbPool,
    pub email_api: MockServer,
    pub api_client: reqwest::Client,
    pub admin: TestUser,
    pub user: TestUser
}

impl TestApp {
    pub async fn delete_orders_admin<Body>(&self, body: Body) -> reqwest::Response
    where 
        Body: Serialize
    {
        self.api_client.delete(format!("http://{}:{}/admin/order",
            self.host,
            self.port,
        ))
        .json(&body)
        .send()
        .await
        .unwrap()
    }

    pub async fn put_orders<Body>(&self, body: Body) -> reqwest::Response
    where 
        Body: Serialize
    {
        self.api_client.put(format!("http://{}:{}/admin/order",
            self.host,
            self.port,
        ))
        .form(&body)
        .send()
        .await
        .unwrap()
    }

    pub async fn get_orders_request(&self, page: i64, limit: i64) -> reqwest::RequestBuilder{
        self.api_client.get(format!("http://{}:{}/order?page={}&limit={}",
            self.host,
            self.port,
            page,
            limit
        ))
    }

    pub async fn get_orders(&self, page: i64, limit: i64) -> reqwest::Response{
        self.get_orders_request(page, limit)
            .await
            .send()
            .await
            .unwrap()
    }

    pub async fn get_inventory(&self, page: i64, limit: i64) -> reqwest::Response{
        self.api_client.get(format!("http://{}:{}/inventory?page={}&limit={}",
            self.host,
            self.port,
            page,
            limit
        ))
        .send()
        .await
        .unwrap()
    }

    pub async fn login_admin(&self){
        let login_request = serde_json::json!({
            "email": self.admin.email,
            "password": self.admin.password
        });

        let login_response = self.api_client.post(format!("http://{}:{}/login", self.host, self.port))
            .form(&login_request)
            .send()
            .await
            .unwrap();

        assert_eq!(login_response.status().as_u16(), 200);
    }
    
    pub async fn post_inventory<Body>(&self, item: Body) -> reqwest::Response
    where 
        Body: Serialize
    {
        self.api_client.post(
            format!("http://{}:{}/admin/inventory",
                self.host,
                self.port
            )
        )
        .form(&item)
        .send()
        .await
        .unwrap()
    }

    fn create_db(settings: &DatabaseSettings) -> DbPool{
        let mut connection = PgConnection::establish(&settings.get_database_url())
                                .expect("Failed to connect to postgres database");

        dbg!(&settings.name);
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

        let admin = TestUser::generate(true, &pool);
        let user = TestUser::generate(false, &pool);

        return TestApp{
            host: application.host,
            port: application.port,
            pool,
            email_api,
            api_client,
            admin,
            user
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


pub async fn create_user_and_login(app: &TestApp){
    let body = serde_json::json!({
        "email" : "amanrao032@gmail.com",
        "name" : "Aman Rao",
        "password" : "testpassword",
        "confirm_password" : "testpassword"
    });

    let guard = Mock::given(path("/email"))
        .and(header_exists("X-Postmark-Server-Token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_api)
        .await;

    app.api_client.post(format!("http://{}:{}/register", app.host, app.port))
            .form(&body)
            .send()
            .await
            .expect("Failed to send request to register endpoint");

    let requests = guard.received_requests().await;
    let body_json: ReceiveEmailRequest = requests[0].body_json().unwrap();

    let link = app.get_confirmation_link(&body_json.text_body);
    
    app.api_client.get(link)
            .send()
            .await
            .expect("Failed to send request to confirm endpoint");

    let login_request = serde_json::json!({
        "email": "amanrao032@gmail.com",
        "password": "testpassword"
    });

    let response = app.api_client.post(format!("http://{}:{}/login", app.host, app.port))
        .form(&login_request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200)
}
