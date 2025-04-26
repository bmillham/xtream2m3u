use self::models::*;
use diesel::prelude::*;
use xtream2m3u::*;

fn main() {
    use self::schema::history::dsl::*;

    let connection = &mut establish_connection();

    let t = add_history(connection, &1, "added");
    println!("t {t:?}");
}
