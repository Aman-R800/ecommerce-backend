
use actix_web::web;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::count;

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
