use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::{Insertable, Queryable};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::order_items;
use crate::schema::users;
use crate::schema::confirmation;
use crate::schema::inventory;
use crate::schema::orders;

#[derive(Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User{
    pub user_id : Uuid,
    pub name: String,
    pub email: String,
    pub password: String,
    pub status: Option<String>,
    pub is_admin: bool
}


#[derive(Queryable, Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = users)]
pub struct UserProfileInfo{
    pub name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub address: Option<String>
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = confirmation)]
pub struct ConfirmationMap{
    pub confirmation_id: Uuid,
    pub user_id: Option<Uuid>
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = inventory)]
pub struct InventoryItem{
    pub item_id: Uuid,
    pub name: String,
    pub amount: Option<i32>,
    pub price: Option<f64>
}

#[derive(Insertable)]
#[diesel(table_name = orders)]
pub struct Order{
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub order_date: DateTime<Utc>,
    pub status: String
}

#[derive(Queryable)]
pub struct OrderQuery{
    pub order_id: Uuid,
    pub user_id: Option<Uuid>,
    pub order_date: Option<DateTime<Utc>>,
    pub status: String
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = order_items)]
pub struct OrderItemModel{
    pub order_item_id: Uuid,
    pub order_id: Uuid,
    pub item_id: Uuid,
    pub quantity: i32
}

#[derive(Queryable)]
pub struct OrderIntermediate{
    pub order_id: Uuid,
    pub user_id: Option<Uuid>,
    pub order_date: Option<DateTime<Utc>>,
    pub status: String,
    pub item_id: Uuid,
    pub quantity: i32
}
