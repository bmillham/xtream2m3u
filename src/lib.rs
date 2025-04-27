pub mod models;
pub mod schema;
use self::models::{
    AddHistory, Categories, Channels, History, NewCategory, NewChannel, NewType, Types,
};
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

pub fn find_or_create_category(conn: &mut SqliteConnection, ctypes_id: &i32, c_name: &str) -> i32 {
    use crate::schema::categories::dsl::*;

    match categories
        .filter(name.eq(c_name))
        .limit(1)
        .load::<Categories>(conn)
    {
        Ok(v) => match v.is_empty() {
            true => match create_category(conn, ctypes_id, c_name, None) {
                Ok(t) => t.id,
                _ => -2,
            },
            false => v[0].id,
        },
        Err(_) => -1,
    }
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

pub fn create_channel(conn: &mut SqliteConnection, categories_id: &i32, name: &str) {
    use crate::schema::channels;

    let new_channel = NewChannel {
        categories_id,
        name,
    };

    let c_id = match diesel::insert_into(channels::table)
        .values(&new_channel)
        .returning(Channels::as_returning())
        .get_result(conn)
    {
        Ok(r) => r.id,
        Err(_) => get_channel_id(conn, name),
    };
    match get_last_channel_change(conn, &c_id).as_str() {
        "" | "deleted" => add_history(conn, &c_id, "added"),
        _ => (),
    };
}

pub fn get_channel_id(conn: &mut SqliteConnection, c_name: &str) -> i32 {
    use crate::schema::channels::dsl::*;

    match channels
        .filter(name.eq(c_name))
        .limit(1)
        .load::<Channels>(conn)
    {
        Ok(r) => match r.is_empty() {
            false => r[0].id,
            true => -2,
        },
        _ => -1,
    }
}

pub fn add_history(conn: &mut SqliteConnection, channels_id: &i32, change_type: &str) {
    use crate::schema::history;

    let new_history = AddHistory {
        channels_id,
        changed: None,
        change_type,
    };

    let _ = diesel::insert_into(history::table)
        .values(&new_history)
        .returning(History::as_returning())
        .get_result(conn);
}

pub fn get_last_channel_change(conn: &mut SqliteConnection, c_id: &i32) -> String {
    use crate::schema::history::dsl::*;

    match history
        .filter(channels_id.eq(c_id))
        .order(changed.desc())
        .limit(1)
        .load::<History>(conn)
    {
        Ok(h) => match h.is_empty() {
            false => h[0].change_type.clone(),
            true => "".to_string(),
        },
        _ => "".to_string(),
    }
}
