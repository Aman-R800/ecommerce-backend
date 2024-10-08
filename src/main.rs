use ecommerce::{configuration::Settings, startup::Application, telemetry::{get_subscriber, init_subscriber}};

#[actix_web::main]
async fn main() -> anyhow::Result<()>{
    let subscriber = get_subscriber("Ecommerce".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = Settings::get();

    let application = Application::new(config)
                        .await
                        .expect("Failed to build application");

    application.server.await?;
    Ok(())
}
