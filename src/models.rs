use diesel::prelude::{Insertable, Queryable};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::users;
use crate::schema::confirmation;
use crate::schema::inventory;

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

#[derive(Queryable, Insertable)]
#[diesel(table_name = inventory)]
pub struct InventoryItem{
    pub item_id: Uuid,
    pub name: String,
    pub amount: Option<i32>,
    pub price: Option<f64>
}
