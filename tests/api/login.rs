use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use ecommerce::models::User;
use wiremock::{matchers::{header_exists, path}, Mock, ResponseTemplate};

use crate::{helpers::{TestApp, LoginResponse}, registration::ReceiveEmailRequest};

#[actix_web::test]
async fn post_login_with_correct_data(){
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


    let login_request = serde_json::json!({
        "email": "amanrao032@gmail.com",
        "password": "testpassword"
    });

    let login_response_body = api_client.post(format!("http://{}:{}/login", app.host, app.port))
        .form(&login_request)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let login_response_json = serde_json::from_str::<LoginResponse>(&login_response_body);
}

#[actix_web::test]
async fn post_login_with_incorrect_data(){
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
            users::status,
            users::is_admin
        ))
        .filter(users::email.eq("amanrao032@gmail.com"))
        .first::<User>(&mut conn)
        .unwrap();

        user.status.unwrap()
    };

    assert_eq!(status, "confirmed");

    let login_request = serde_json::json!({
        "email": "amanrao032@gmail.com",
        "password": "wrongpassword"
    });

    let login_response = api_client.post(format!("http://{}:{}/login", app.host, app.port))
        .form(&login_request)
        .send()
        .await
        .unwrap();

    dbg!(&login_response);
    assert_eq!(login_response.status().as_u16(), 401);
    dbg!(&login_response.text().await.unwrap());
}
