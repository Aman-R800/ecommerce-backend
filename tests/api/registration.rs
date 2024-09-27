
use actix_web::web;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::count;
use ecommerce::email_client::SendEmailRequest;
use ecommerce::models::User;
use serde::Deserialize;
use wiremock::matchers::{header_exists, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;

#[actix_web::test]
async fn post_registration_without_form_data_fails() {
    let app = TestApp::spawn_app().await;
    let api_client = reqwest::Client::new();

    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 415)
}

#[actix_web::test]
async fn post_registration_with_valid_form_data(){
    let app = TestApp::spawn_app().await;
    let body = serde_json::json!({
        "email" : "amanrao032@gmail.com",
        "name" : "Aman Rao",
        "password" : "testpassword",
        "confirm_password" : "testpassword"
    });

    Mock::given(path("/email"))
        .and(header_exists("X-Postmark-Server-Token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_api)
        .await;

    let api_client = reqwest::Client::new();
    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn post_registration_passwords_not_matching(){
    let app = TestApp::spawn_app().await;
    let body = serde_json::json!({
        "email" : "amanrao032@gmail.com",
        "name" : "Aman Rao",
        "password" : "testpassword",
        "confirm_password" : "differentpassword"
    });

    let api_client = reqwest::Client::new();
    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_web::test]
async fn post_registration_adds_user_to_db(){
    let app = TestApp::spawn_app().await;
    let body = serde_json::json!({
        "email" : "amanrao032@gmail.com",
        "name" : "Aman Rao",
        "password" : "testpassword",
        "confirm_password" : "testpassword"
    });

    Mock::given(path("/email"))
        .and(header_exists("X-Postmark-Server-Token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_api)
        .await;

    let api_client = reqwest::Client::new();
    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 200);

    let mut conn = app.pool.get().unwrap();

    let rows: i64 = web::block(move ||{
        use ecommerce::schema::users::dsl::*;
        
        users.filter(email.eq("amanrao032@gmail.com"))
            .select(count(email))
            .first(&mut conn)
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(rows, 1)
}

#[actix_web::test]
async fn post_registration_sends_confirmation_mail(){
    let app = TestApp::spawn_app().await;
    let body = serde_json::json!({
        "email" : "amanrao032@gmail.com",
        "name" : "Aman Rao",
        "password" : "testpassword",
        "confirm_password" : "testpassword"
    });

    Mock::given(path("/email"))
        .and(header_exists("X-Postmark-Server-Token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_api)
        .await;


    let api_client = reqwest::Client::new();
    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn get_confirm_confirms_subscription(){
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

    let api_client = reqwest::Client::new();
    let response = api_client.post(format!("http://{}:{}/register", app.host, app.port))
                    .form(&body)
                    .send()
                    .await
                    .expect("Failed to send request to register endpoint");

    assert_eq!(response.status().as_u16(), 200);

    let requests = guard.received_requests().await;
    let body_json: ReceiveEmailRequest = requests[0].body_json().unwrap();

    let link = app.get_confirmation_link(&body_json.text_body);
    
    
    let confirm_response = api_client.get(link)
                    .send()
                    .await
                    .expect("Failed to send request to confirm endpoint");

    assert_eq!(confirm_response.status().as_u16(), 200);

    let status = {
        use ecommerce::schema::users;

        let mut conn = app.pool.get().unwrap();

        let user: User = users::table.select((
            users::user_id,
            users::name,
            users::email,
            users::password,
            users::status
        ))
        .filter(users::email.eq("amanrao032@gmail.com"))
        .first::<User>(&mut conn)
        .unwrap();

        user.status.unwrap()
    };

    assert_eq!(status, "confirmed")
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReceiveEmailRequest{
    pub from: String,
    pub to: String,
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}
