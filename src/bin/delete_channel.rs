use self::models::*;
use chrono::NaiveDateTime;
use diesel::dsl::now;
use diesel::prelude::*;
use xtream2m3u::*;

fn main() {
    use self::schema::channels::dsl::*;

    let connection = &mut establish_connection();
    let chan_id = "test";

    let res = channels
        .filter(name.eq(chan_id))
        .load::<Channels>(connection);

    println!("chan {res:?}");

    let r = diesel::update(channels)
        .filter(name.eq(chan_id))
        //.set(deleted.eq(None::<NaiveDateTime>))
        .set(deleted.eq(now))
        .execute(connection);
    println!("{r:?}");
}
