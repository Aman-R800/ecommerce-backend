use diesel::prelude::{Insertable, Queryable};
use uuid::Uuid;

use crate::schema::users;


#[derive(Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User{
    pub user_id : Uuid,
    pub name: String,
    pub email: String,
    pub password: String
}
