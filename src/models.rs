use diesel::prelude::{Insertable, Queryable};
use uuid::Uuid;

use crate::schema::users;
use crate::schema::confirmation;

#[derive(Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User{
    pub user_id : Uuid,
    pub name: String,
    pub email: String,
    pub password: String,
    pub status: Option<String>
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = confirmation)]
pub struct ConfirmationMap{
    pub confirmation_id: Uuid,
    pub user_id: Option<Uuid>
}
