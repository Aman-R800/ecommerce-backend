use crate::helpers::TestApp;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use ecommerce::schema::inventory;

#[actix_web::test]
pub async fn add_item_to_inventory(){
    let app = TestApp::spawn_app().await;

    app.login_admin().await;

    let item = serde_json::json!({
        "name" : "example item",
        "amount" : "500",
        "price" : "500"
    });

    let response = app.post_inventory(item).await;
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
