use self::models::*;
use diesel::prelude::*;
use xtream2m3u::*;

fn main() {
    use self::schema::history::dsl::*;

    let connection = &mut establish_connection();

    let c1 = "US: FOX 44 (WFFF) BURLINGTON HD";
    //let c1 = "NF - The Two Popes";
    let i = get_channel_id(connection, c1);
    println!("{c1} id {i}");
    let x = get_last_channel_change(connection, &i);
    println!("x {x}");
}
