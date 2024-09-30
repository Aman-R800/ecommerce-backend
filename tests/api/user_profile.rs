use crate::helpers::TestApp;

#[actix_web::test]
async fn get_profile_without_logged_in_user(){
    let app = TestApp::spawn_app().await;

    let api_client = reqwest::Client::new();
    let response = api_client.get(format!("http://{}:{}/user/profile", app.host, app.port))
                    .send()
                    .await
                    .expect("Failed to send request to user profile endpoint");

    assert_eq!(response.status().as_u16(), 403)
}
