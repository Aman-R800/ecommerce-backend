use ecommerce::models::UserProfileInfo;
use wiremock::{matchers::{header_exists, path}, Mock, ResponseTemplate};

use crate::{helpers::{LoginResponse, TestApp}, registration::ReceiveEmailRequest};

#[actix_web::test]
async fn get_profile_without_logged_in_user(){
    let app = TestApp::spawn_app().await;

    let api_client = reqwest::Client::new();
    let response = api_client.get(format!("http://{}:{}/user/profile", app.host, app.port))
                    .send()
                    .await
                    .expect("Failed to send request to user profile endpoint");

    assert_eq!(response.status().as_u16(), 401)
}

#[actix_web::test]
async fn get_profile_with_logged_in_user(){
    let app = TestApp::spawn_app().await;
    
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

    let response = app.api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 200);

    let requests = guard.received_requests().await;
    let body_json: ReceiveEmailRequest = requests[0].body_json().unwrap();

    let link = app.get_confirmation_link(&body_json.text_body);
    
    
    let confirm_response = app.api_client.get(link)
                    .send()
                    .await
                    .expect("Failed to send request to confirm endpoint");

    assert_eq!(confirm_response.status().as_u16(), 200);

    let login_request = serde_json::json!({
        "email": "amanrao032@gmail.com",
        "password": "testpassword"
    });

    let login_response_body = app.api_client.post(format!("http://{}:{}/login", app.host, app.port))
        .form(&login_request)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let login_response_json: LoginResponse = serde_json::from_str(&login_response_body).unwrap();

    let response = app.api_client.get(format!("http://{}:{}/user/profile", app.host, app.port))
                    .bearer_auth(login_response_json.access_token)
                    .send()
                    .await
                    .expect("Failed to send request to user profile endpoint");

    assert_eq!(response.status().as_u16(), 200);

    let body: UserProfileInfo = response.json().await.unwrap();
    assert_eq!(body.name, "Aman Rao");
    assert_eq!(body.email, "amanrao032@gmail.com");
    assert_eq!(body.phone_number, None);
    assert_eq!(body.address, None);
}


#[actix_web::test]
async fn post_profile_with_logged_in_user(){
    let app = TestApp::spawn_app().await;
    
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

    let _response = app.api_client.post(format!("http://{}:{}/register", app.host, app.port))
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

    let login_response_body = app.api_client.post(format!("http://{}:{}/login", app.host, app.port))
        .form(&login_request)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let login_response_json: LoginResponse = serde_json::from_str(&login_response_body).unwrap();

    let profile_body = serde_json::json!({
        "phone_number": "8927401349",
        "address": "test, address, 45585, India"
    });

    let post_profile_response = app.api_client.post(format!("http://{}:{}/user/profile", app.host, app.port))
        .bearer_auth(&login_response_json.access_token)
        .form(&profile_body)
        .send()
        .await
        .unwrap();

    assert_eq!(post_profile_response.status().as_u16(), 200);


    let response = app.api_client.get(format!("http://{}:{}/user/profile", app.host, app.port))
                    .bearer_auth(&login_response_json.access_token)
                    .send()
                    .await
                    .expect("Failed to send request to user profile endpoint");


    let body: UserProfileInfo = response.json().await.unwrap();

    assert_eq!(body.phone_number, Some("8927401349".to_string()));
    assert_eq!(body.address, Some("test, address, 45585, India".to_string()))
}
