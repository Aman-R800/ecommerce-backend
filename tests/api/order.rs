use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use ecommerce::{models::{InventoryItem, Order, OrderQuery}, db_interaction::OrderWithItems, schema::{inventory, orders}};
use uuid::Uuid;

use crate::helpers::{create_user_and_login, TestApp};

#[actix_web::test]
pub async fn post_order_creates_order(){
    let app = TestApp::spawn_app().await;

    let inventory_items = vec![
        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 1".to_string(),
            amount: Some(50_i32),
            price: Some(47_f64)
        },

        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 2".to_string(),
            amount: Some(75_i32),
            price: Some(100_f64)
        },

        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 3".to_string(),
            amount: Some(28_i32),
            price: Some(60_f64)
        }
    ];

    let mut conn = app.pool.get().unwrap();
    for item in inventory_items.iter(){
        diesel::insert_into(inventory::table)
            .values(item)
            .execute(&mut conn)
            .unwrap();
    }

    let access_token = create_user_and_login(&app).await;

    let order_data = serde_json::json!([
        {
            "item_id": inventory_items[0].item_id,
            "amount": 5_i32
        },


        {
            "item_id": inventory_items[1].item_id,
            "amount": 8_i32
        },


        {
            "item_id": inventory_items[2].item_id,
            "amount": 40_i32
        }
    ]);

    let response = app.api_client.post(format!("http://{}:{}/user/order", app.host, app.port))
        .bearer_auth(access_token)
        .json(&order_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.json::<Vec<Uuid>>().await.unwrap();

    assert_eq!(body.len(), 2);
    
    let ideal = vec![inventory_items[0].item_id, inventory_items[1].item_id];
    for item_id in body.iter(){
        assert!(ideal.contains(item_id))
    }
}

#[actix_web::test]
pub async fn get_order_returns_orders(){
    let app = TestApp::spawn_app().await;

    let inventory_items = vec![
        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 1".to_string(),
            amount: Some(50_i32),
            price: Some(47_f64)
        },

        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 2".to_string(),
            amount: Some(75_i32),
            price: Some(100_f64)
        },

        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 3".to_string(),
            amount: Some(28_i32),
            price: Some(60_f64)
        }
    ];

    let mut conn = app.pool.get().unwrap();
    for item in inventory_items.iter(){
        diesel::insert_into(inventory::table)
            .values(item)
            .execute(&mut conn)
            .unwrap();
    }

    let access_token = create_user_and_login(&app).await;

    let order_data = serde_json::json!([
        {
            "item_id": inventory_items[0].item_id,
            "amount": 5_i32
        },


        {
            "item_id": inventory_items[1].item_id,
            "amount": 8_i32
        },


        {
            "item_id": inventory_items[2].item_id,
            "amount": 40_i32
        }
    ]);

    let response = app.api_client.post(format!("http://{}:{}/user/order", app.host, app.port))
        .bearer_auth(&access_token)
        .json(&order_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.json::<Vec<Uuid>>().await.unwrap();

    assert_eq!(body.len(), 2);
    
    let ideal = vec![inventory_items[0].item_id, inventory_items[1].item_id];
    for item_id in body.iter(){
        assert!(ideal.contains(item_id))
    }

    
    let get_orders = app.get_orders(1, 10, &access_token)
                        .await
                        .json::<Vec<OrderWithItems>>()
                        .await
                        .unwrap();

    assert_eq!(get_orders.len(), 1);
    assert_eq!(get_orders[0].items.len(), 2);
}

#[actix_web::test]
async fn concurrent_orders_is_consistent(){
    let app = TestApp::spawn_app().await;

    let inventory_items = vec![
        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 1".to_string(),
            amount: Some(50_i32),
            price: Some(47_f64)
        },

        InventoryItem{
            item_id: Uuid::new_v4(),
            name: "item 2".to_string(),
            amount: Some(2_i32),
            price: Some(100_f64)
        },
    ];

    let mut conn = app.pool.get().unwrap();
    for item in inventory_items.iter(){
        diesel::insert_into(inventory::table)
            .values(item)
            .execute(&mut conn)
            .unwrap();
    }

    let access_token = create_user_and_login(&app).await;

    let order_data = serde_json::json!([
        {
            "item_id": inventory_items[0].item_id,
            "amount": 5_i32
        },


        {
            "item_id": inventory_items[1].item_id,
            "amount": 2_i32
        },
    ]);

    let order_data2 = serde_json::json!([
        {
            "item_id": inventory_items[0].item_id,
            "amount": 5_i32
        },


        {
            "item_id": inventory_items[1].item_id,
            "amount": 1_i32
        },
    ]);

    let response1 = app.api_client.post(format!("http://{}:{}/user/order", app.host, app.port))
        .bearer_auth(&access_token)
        .json(&order_data)
        .send();

    let response2 = app.api_client.post(format!("http://{}:{}/user/order", app.host, app.port))
        .bearer_auth(&access_token)
        .json(&order_data2)
        .send();

    let (first, second) = tokio::join!(response1, response2);
    let first = first.unwrap().json::<Vec<Uuid>>().await.unwrap();
    let second = second.unwrap().json::<Vec<Uuid>>().await.unwrap();

    assert_eq!(first.len() + second.len(), 3)
}

#[actix_web::test]
async fn update_order_status(){
    let app = TestApp::spawn_app().await;
    let mut conn = app.pool.get().unwrap();

    let order_id = Uuid::new_v4();
    let test_order = Order{
        order_id: order_id.clone(),
        user_id: app.user.user_id,
        order_date: Utc::now(),
        status: "pending".to_string()
    };
    
    diesel::insert_into(orders::table)
        .values(&test_order)
        .execute(&mut conn)
        .unwrap();

    let access_token = app.login_admin().await;

    let put_order_request = serde_json::json!({
        "order_id": order_id,
        "status": "shipped"
    });

    let response = app.put_orders(put_order_request, &access_token).await;
    assert_eq!(response.status().as_u16(), 200);

    let updated_order: OrderQuery = orders::table
                            .filter(orders::order_id.eq(order_id))
                            .get_result::<OrderQuery>(&mut conn)
                            .unwrap();

    assert_eq!(updated_order.status, "shipped")
}



#[actix_web::test]
async fn delete_order_works_for_admin(){
    let app = TestApp::spawn_app().await;
    let mut conn = app.pool.get().unwrap();

    let order_id = Uuid::new_v4();
    let test_order = Order{
        order_id: order_id.clone(),
        user_id: app.user.user_id,
        order_date: Utc::now(),
        status: "pending".to_string()
    };
    
    diesel::insert_into(orders::table)
        .values(&test_order)
        .execute(&mut conn)
        .unwrap();

    let access_token = app.login_admin().await;

    let delete_order_request = serde_json::json!({
        "order_id": order_id,
    });

    let response = app.delete_orders_admin(delete_order_request, &access_token).await;
    assert_eq!(response.status().as_u16(), 200);


    let orders: Vec<OrderQuery> = orders::table
                            .filter(orders::order_id.eq(order_id))
                            .get_results::<OrderQuery>(&mut conn)
                            .unwrap();

    assert_eq!(orders.len(), 0)
}
