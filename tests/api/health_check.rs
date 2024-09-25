use crate::helpers::TestApp;

#[actix_web::test]
async fn check_health_route(){
    let app = TestApp::spawn_app().await;
    let url = app.get_app_url();

    dbg!(&url);
    let response = reqwest::get(format!("{}/health", url))
                    .await
                    .expect("Failed to get response");

    assert_eq!(response.status().as_u16(), 200)
}
