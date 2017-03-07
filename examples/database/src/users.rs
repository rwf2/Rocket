// use self::schema::users;
// use self::schema::users::dsl::users as all_users;

// mod schema {
//     infer_schema!("env:DATABASE_URL");
// }

// #[table_name = "users"]
// #[derive(Deserialize, Queryable, Insertable, FromForm, Debug, Clone)]
// pub struct User {
//     id: i32,
//     pub username: String,
//     pub password: String
// }

// impl User {
//     pub fn all(conn: &PgConnection) -> Vec<User> {
//         all_users.order(users::id.desc()).load::<User>(conn).unwrap()
//     }

//     pub fn insert(&self, conn: &PgConnection) -> bool {
//         diesel::insert(self).into(users::table).execute(conn).is_ok()
//     }

//     pub fn delete_with_id(id: i32, conn: &PgConnection) -> bool {
//         diesel::delete(all_users.find(id)).execute(conn).is_ok()
//     }
// }
