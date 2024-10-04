// @generated automatically by Diesel CLI.

diesel::table! {
    confirmation (confirmation_id) {
        confirmation_id -> Uuid,
        user_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    inventory (item_id) {
        item_id -> Uuid,
        name -> Text,
        amount -> Nullable<Int4>,
        price -> Nullable<Float8>,
    }
}

diesel::table! {
    order_items (order_item_id) {
        order_item_id -> Uuid,
        order_id -> Uuid,
        item_id -> Uuid,
        quantity -> Int4,
    }
}

diesel::table! {
    orders (order_id) {
        order_id -> Uuid,
        user_id -> Nullable<Uuid>,
        order_date -> Nullable<Timestamptz>,
        status -> Text,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Uuid,
        name -> Text,
        email -> Text,
        password -> Text,
        status -> Nullable<Text>,
        #[max_length = 10]
        phone_number -> Nullable<Varchar>,
        address -> Nullable<Text>,
        is_admin -> Bool,
    }
}

diesel::joinable!(confirmation -> users (user_id));
diesel::joinable!(order_items -> inventory (item_id));
diesel::joinable!(order_items -> orders (order_id));
diesel::joinable!(orders -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    confirmation,
    inventory,
    order_items,
    orders,
    users,
);
