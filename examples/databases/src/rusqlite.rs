use rocket::{Rocket, Build};
use rocket::fairing::AdHoc;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::response::{Debug, status::Created};

use rocket_sync_db_pools::rusqlite;

use self::rusqlite::params;

#[database("rusqlite")]
struct Db(rusqlite::Connection);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Post {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    title: String,
    text: String,
}

type Result<T, E = Debug<rusqlite::Error>> = std::result::Result<T, E>;

#[post("/", data = "<post>")]
async fn create(db: Db, post: Json<Post>) -> Result<Created<Json<Post>>> {
    let item = post.clone();
    db.run(move |conn| {
        conn.execute("INSERT INTO posts (title, text) VALUES (?1, ?2)",
            params![item.title, item.text])
    }).await?;

    Ok(Created::new("/").body(post))
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<i64>>> {
    let ids = db.run(|conn| {
        conn.prepare("SELECT id FROM posts")?
            .query_map(params![], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn read(db: Db, id: i64) -> Option<Json<Post>> {
    let post = db.run(move |conn| {
        conn.query_row("SELECT id, title, text FROM posts WHERE id = ?1", params![id],
            |r| Ok(Post { id: Some(r.get(0)?), title: r.get(1)?, text: r.get(2)? }))
    }).await.ok()?;

    Some(Json(post))
}

#[delete("/<id>")]
async fn delete(db: Db, id: i64) -> Result<Option<()>> {
    let affected = db.run(move |conn| {
        conn.execute("DELETE FROM posts WHERE id = ?1", params![id])
    }).await?;

    Ok((affected == 1).then(|| ()))
}

#[delete("/")]
async fn destroy(db: Db) -> Result<()> {
    db.run(move |conn| conn.execute("DELETE FROM posts", params![])).await?;

    Ok(())
}

async fn init_db(rocket: Rocket<Build>) -> Rocket<Build> {
    Db::get_one(&rocket).await
        .expect("database mounted")
        .run(|conn| {
            conn.execute(r#"
                CREATE TABLE posts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title VARCHAR NOT NULL,
                    text VARCHAR NOT NULL,
                    published BOOLEAN NOT NULL DEFAULT 0
                )"#, params![])
        }).await
        .expect("can init rusqlite DB");

    rocket
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Rusqlite Stage", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Rusqlite Init", init_db))
            .mount("/rusqlite", routes![list, create, read, delete, destroy])
    })
}
