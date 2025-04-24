// @generated automatically by Diesel CLI.

diesel::table! {
    categories (id) {
        id -> Integer,
        types_id -> Integer,
        name -> Text,
        added -> Nullable<Timestamp>,
    }
}

diesel::table! {
    types (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(categories -> types (types_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    types,
);
