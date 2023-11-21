use rocket::serde::Serialize;
use diesel::{self, result::QueryResult, prelude::*};
use rocket_db_pools::diesel::RunQueryDsl;

mod schema {
    diesel::table! {
        tasks {
            id -> Nullable<Integer>,
            description -> Text,
            completed -> Bool,
        }
    }
}

use self::schema::tasks;

type DbConn = rocket_db_pools::diesel::AsyncPgConnection;

#[derive(Serialize, Queryable, Insertable, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tasks)]
pub struct Task {
    #[serde(skip_deserializing)]
    pub id: Option<i32>,
    pub description: String,
    pub completed: bool
}

#[derive(Debug, FromForm)]
pub struct Todo {
    pub description: String,
}

impl Task {
    pub async fn all(conn: &mut DbConn) -> QueryResult<Vec<Task>> {
        tasks::table.order(tasks::id.desc()).load::<Task>(conn).await
    }

    /// Returns the number of affected rows: 1.
    pub async fn insert(todo: Todo, conn: &mut DbConn) -> QueryResult<usize> {
        let t = Task { id: None, description: todo.description, completed: false };
        diesel::insert_into(tasks::table).values(&t).execute(conn).await
    }

    /// Returns the number of affected rows: 1.
    pub async fn toggle_with_id(id: i32, conn: &mut DbConn) -> QueryResult<usize> {
        let task = tasks::table.filter(tasks::id.eq(id)).get_result::<Task>(conn).await?;
        let new_status = !task.completed;
        let updated_task = diesel::update(tasks::table.filter(tasks::id.eq(id)));
        updated_task.set(tasks::completed.eq(new_status)).execute(conn).await
    }

    /// Returns the number of affected rows: 1.
    pub async fn delete_with_id(id: i32, conn: &mut DbConn) -> QueryResult<usize> {
        diesel::delete(tasks::table)
            .filter(tasks::id.eq(id))
            .execute(conn)
            .await
    }

    /// Returns the number of affected rows.
    #[cfg(test)]
    pub async fn delete_all(conn: &mut DbConn) -> QueryResult<usize> {
        diesel::delete(tasks::table).execute(conn).await
    }
}
