use ecommerce::{configuration::Settings, startup::Application, telemetry::{get_subscriber, init_subscriber}};

#[actix_web::main]
async fn main() -> anyhow::Result<()>{
    let subscriber = get_subscriber("Ecommerce".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = Settings::get();

    let mut application = Application::new(config.application);
    application.get_server()?.await?;
    Ok(())
}
