use rocket::fairing::AdHoc;
use rocket::response::{Debug, status::Created};
use rocket::serde::{Serialize, Deserialize, json::Json};

use rocket_db_pools::{Database, Connection};
use rocket_db_pools::diesel::{MysqlPool, prelude::*};

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Database)]
#[database("diesel_mysql")]
struct Db(MysqlPool);

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = posts)]
struct Post {
    #[serde(skip_deserializing)]
    id: Option<i64>,
    title: String,
    text: String,
    #[serde(skip_deserializing)]
    published: bool,
}

diesel::table! {
    posts (id) {
        id -> Nullable<BigInt>,
        title -> Text,
        text -> Text,
        published -> Bool,
    }
}

#[post("/", data = "<post>")]
async fn create(mut db: Connection<Db>, mut post: Json<Post>) -> Result<Created<Json<Post>>> {
    diesel::sql_function!(fn last_insert_id() -> BigInt);

    let post = db.transaction(|mut conn| Box::pin(async move {
        diesel::insert_into(posts::table)
            .values(&*post)
            .execute(&mut conn)
            .await?;

        post.id = Some(posts::table
            .select(last_insert_id())
            .first(&mut conn)
            .await?);

        Ok::<_, diesel::result::Error>(post)
    })).await?;

    Ok(Created::new("/").body(post))
}

#[get("/")]
async fn list(mut db: Connection<Db>) -> Result<Json<Vec<Option<i64>>>> {
    let ids = posts::table
        .select(posts::id)
        .load(&mut db)
        .await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn read(mut db: Connection<Db>, id: i64) -> Option<Json<Post>> {
    posts::table
        .filter(posts::id.eq(id))
        .first(&mut db)
        .await
        .map(Json)
        .ok()
}

#[delete("/<id>")]
async fn delete(mut db: Connection<Db>, id: i64) -> Result<Option<()>> {
    let affected = diesel::delete(posts::table)
        .filter(posts::id.eq(id))
        .execute(&mut db)
        .await?;

    Ok((affected == 1).then(|| ()))
}

#[delete("/")]
async fn destroy(mut db: Connection<Db>) -> Result<()> {
    diesel::delete(posts::table).execute(&mut db).await?;
    Ok(())
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Diesel MySQL Stage", |rocket| async {
        rocket.attach(Db::init())
            .mount("/diesel-async", routes![list, read, create, delete, destroy])
    })
}
