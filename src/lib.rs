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

pub fn find_or_create_type(conn: &mut SqliteConnection, t_name: &str) -> i32 {
    use crate::schema::types::dsl::*;

    match types.filter(name.eq(t_name)).limit(1).load::<Types>(conn) {
        Ok(v) => match v.is_empty() {
            true => match create_type(conn, t_name) {
                Ok(t) => t.id,
                _ => -2,
            },
            false => v[0].id,
        },
        Err(e) => {
            println!("Error creating type: {e:?}");
            -1
        }
    }
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
