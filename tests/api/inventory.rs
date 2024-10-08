use crate::helpers::TestApp;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use ecommerce::{models::InventoryItem, schema::inventory};

#[actix_web::test]
pub async fn add_item_to_inventory(){
    let app = TestApp::spawn_app().await;

    let access_token = app.login_admin().await;

    let item = serde_json::json!({
        "name" : "example item",
        "amount" : "500",
        "price" : "500"
    });

    let response = app.post_inventory(item, access_token).await;
    assert_eq!(response.status().as_u16(), 200);

    let mut conn = app.pool.get().unwrap();

    let response: i64 = inventory::table
        .filter(
            inventory::name.eq("example item")
                .and(inventory::amount.eq(500 as i32))
                .and(inventory::price.eq(500 as f64))
        )
        .count()
        .get_result::<i64>(&mut conn)
        .unwrap();

    assert_eq!(response, 1);
}


#[actix_web::test]
pub async fn get_item_from_inventory(){
    let app = TestApp::spawn_app().await;

    let access_token = app.login_admin().await;

    let item = serde_json::json!({
        "name" : "example item",
        "amount" : "500",
        "price" : "500"
    });

    let _post_response = app.post_inventory(item, access_token).await;

    let mut conn = app.pool.get().unwrap();
    let _response: i64 = inventory::table
        .filter(
            inventory::name.eq("example item")
                .and(inventory::amount.eq(500 as i32))
                .and(inventory::price.eq(500 as f64))
        )
        .count()
        .get_result::<i64>(&mut conn)
        .unwrap();


    let get_response: Vec<InventoryItem> = app.get_inventory(1, 5)
        .await
        .json::<Vec<InventoryItem>>()
        .await
        .unwrap();

    assert_eq!(get_response[0].name, "example item");
    assert_eq!(get_response[0].amount, Some(500_i32));
    assert_eq!(get_response[0].price, Some(500_f64));
}
