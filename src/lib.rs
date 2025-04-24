pub mod models;
pub mod schema;
use self::models::{Categories, NewCategory, NewType, Types};
use chrono::NaiveDateTime;
use diesel::prelude::*;

pub fn establish_connection() -> SqliteConnection {
    let database_url = "channels.db";
    SqliteConnection::establish(database_url)
        .unwrap_or_else(|e| panic!("Error {e} connecting to {database_url}"))
}

pub fn create_type(
    conn: &mut SqliteConnection,
    name: &str,
) -> Result<Types, diesel::result::Error> {
    use crate::schema::types;

    let new_type = NewType { name };

    diesel::insert_into(types::table)
        .values(&new_type)
        .returning(Types::as_returning())
        .get_result(conn)
}

pub fn create_category(
    conn: &mut SqliteConnection,
    types_id: &i32,
    name: &str,
    added: Option<NaiveDateTime>,
) -> Result<Categories, diesel::result::Error> {
    use crate::schema::categories;

    let new_category = NewCategory {
        types_id,
        name,
        added,
    };

    diesel::insert_into(categories::table)
        .values(&new_category)
        .returning(Categories::as_returning())
        .get_result(conn)
}
