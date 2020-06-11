use diesel::{self, result::QueryResult, prelude::*};

mod schema {
    table! {
        tasks {
            id -> Nullable<Integer>,
            description -> Text,
            completed -> Bool,
        }
    }
}

use self::schema::tasks;
use self::schema::tasks::dsl::{tasks as all_tasks, completed as task_completed};

#[table_name="tasks"]
#[derive(Serialize, Queryable, Insertable, Debug, Clone)]
pub struct Task {
    pub id: Option<i32>,
    pub description: String,
    pub completed: bool
}

#[derive(Debug, FromForm)]
pub struct Todo {
    pub description: String,
}

impl Task {
    pub fn all(conn: &SqliteConnection) -> QueryResult<Vec<Task>> {
        all_tasks.order(tasks::id.desc()).load::<Task>(conn)
    }

    /// Returns the number of affected rows: 1.
    pub fn insert(todo: Todo, conn: &SqliteConnection) -> QueryResult<usize> {
        let t = Task { id: None, description: todo.description, completed: false };
        diesel::insert_into(tasks::table).values(&t).execute(conn)
    }

    /// Returns the number of affected rows: 1.
    pub fn toggle_with_id(id: i32, conn: &SqliteConnection) -> QueryResult<usize> {
        let task = all_tasks.find(id).get_result::<Task>(conn)?;
        let new_status = !task.completed;
        let updated_task = diesel::update(all_tasks.find(id));
        updated_task.set(task_completed.eq(new_status)).execute(conn)
    }

    /// Returns the number of affected rows: 1.
    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> QueryResult<usize> {
        diesel::delete(all_tasks.find(id)).execute(conn)
    }

    /// Returns the number of affected rows.
    #[cfg(test)]
    pub fn delete_all(conn: &SqliteConnection) -> QueryResult<usize> {
        diesel::delete(all_tasks).execute(conn)
    }
}
