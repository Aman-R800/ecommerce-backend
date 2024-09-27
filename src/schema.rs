// @generated automatically by Diesel CLI.

diesel::table! {
    confirmation (confirmation_id) {
        confirmation_id -> Uuid,
        user_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Uuid,
        name -> Text,
        email -> Text,
        password -> Text,
        status -> Nullable<Text>,
    }
}

diesel::joinable!(confirmation -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    confirmation,
    users,
);
