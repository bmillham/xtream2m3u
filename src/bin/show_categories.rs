use self::models::*;
use diesel::prelude::*;
use xtream2m3u::*;

fn main() {
    use self::schema::categories::dsl::*;

    let connection = &mut establish_connection();

    // Use select to only return the name column
    /*let res = categories
        .select(name)
        .order_by(name.desc())
        .load::<String>(connection)
        .unwrap();
    for c in res {
        println!("{c:?}");
    }*/

    // No select to get all columns
    let res = categories.order_by(name).load::<Categories>(connection);

    if let Ok(r) = res {
        for c in r {
            println!("{}", c.name);
        }
    };
}
