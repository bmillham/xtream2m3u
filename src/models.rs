use crate::schema::{categories, channels, history, types};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Deserialize;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = crate::schema::types)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Types {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = types)]
pub struct NewType<'a> {
    pub name: &'a str,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Deserialize)]
#[diesel(table_name = crate::schema::categories)]
#[diesel(belongs_to(Types))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Categories {
    pub id: i32,
    pub types_id: i32,
    pub name: String,
    pub added: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = categories)]
pub struct NewCategory<'a> {
    pub types_id: &'a i32,
    pub name: &'a str,
    pub added: Option<NaiveDateTime>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Deserialize)]
#[diesel(table_name = crate::schema::channels)]
#[diesel(belongs_to(Categories))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Channels {
    pub id: i32,
    pub categories_id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = channels)]
pub struct NewChannel<'a> {
    pub categories_id: &'a i32,
    pub name: &'a str,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Deserialize)]
#[diesel(table_name = crate::schema::history)]
#[diesel(belongs_to(Channels))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct History {
    pub id: i32,
    pub channels_id: i32,
    pub changed: Option<NaiveDateTime>,
    pub change_type: String,
}

#[derive(Insertable)]
#[diesel(table_name = history)]
pub struct AddHistory<'a> {
    pub channels_id: &'a i32,
    pub changed: Option<NaiveDateTime>,
    pub change_type: &'a str,
}
