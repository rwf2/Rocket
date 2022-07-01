use rocket::{Rocket, Build};
use rocket::fairing::AdHoc;
use rocket::response::{Debug, status::Created};
use rocket::serde::{Serialize, Deserialize, json::Json};

use rocket_sync_db_pools::diesel;

use self::diesel::prelude::*;

#[database("diesel")]
struct Db(diesel::SqliteConnection);

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[table_name="posts"]
struct Post {
    #[serde(skip_deserializing)]
    id: Option<i32>,
    title: String,
    text: String,
    #[serde(skip_deserializing)]
    published: bool,
}

table! {
    posts (id) {
        id -> Nullable<Integer>,
        title -> Text,
        text -> Text,
        published -> Bool,
    }
}

#[post("/", data = "<post>")]
async fn create(db: Db, post: Json<Post>) -> Result<Created<Json<Post>>> {
    let post_value = post.clone();
    db.run(move |conn| {
        diesel::insert_into(posts::table)
            .values(&*post_value)
            .execute(conn)
    }).await?;

    Ok(Created::new("/").body(post))
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<Option<i32>>>> {
    let ids: Vec<Option<i32>> = db.run(move |conn| {
        posts::table
            .select(posts::id)
            .load(conn)
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn read(db: Db, id: i32) -> Option<Json<Post>> {
    db.run(move |conn| {
        posts::table
            .filter(posts::id.eq(id))
            .first(conn)
    }).await.map(Json).ok()
}

#[delete("/<id>")]
async fn delete(db: Db, id: i32) -> Result<Option<()>> {
    let affected = db.run(move |conn| {
        diesel::delete(posts::table)
            .filter(posts::id.eq(id))
            .execute(conn)
    }).await?;

    Ok((affected == 1).then(|| ()))
}

#[delete("/")]
async fn destroy(db: Db) -> Result<()> {
    db.run(move |conn| diesel::delete(posts::table).execute(conn)).await?;

    Ok(())
}

async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    // This macro from `diesel_migrations` defines an `embedded_migrations`
    // module containing a function named `run` that runs the migrations in the
    // specified directory, initializing the database.
    embed_migrations!("db/diesel/migrations");

    let conn = Db::get_one(&rocket).await.expect("database connection");
    conn.run(|c| embedded_migrations::run(c)).await.expect("diesel migrations");

    rocket
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Diesel SQLite Stage", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/diesel", routes![list, read, create, delete, destroy])
    })
}
