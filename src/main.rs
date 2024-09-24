use ecommerce::{configuration::Settings, startup::Application};

#[actix_web::main]
async fn main() -> anyhow::Result<()>{
    let config = Settings::get();

    let application = Application::new(config);
    application.get_server()?.await?;
    Ok(())
}
