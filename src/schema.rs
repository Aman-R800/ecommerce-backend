// @generated automatically by Diesel CLI.

diesel::table! {
    users (user_id) {
        user_id -> Uuid,
        name -> Text,
        email -> Text,
        password -> Text,
    }
}
