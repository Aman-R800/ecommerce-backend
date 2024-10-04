use diesel::RunQueryDsl;
use ecommerce::{models::InventoryItem, schema::inventory};
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

    create_user_and_login(&app).await;

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
