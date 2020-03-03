use diesel::{self, result::Error as DieselError, prelude::*};
type Result<T> = std::result::Result<T, DieselError>;

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

#[derive(FromForm)]
pub struct Todo {
    pub description: String,
}

impl Task {
    pub fn all(conn: &SqliteConnection) -> Result<Vec<Task>> {
        all_tasks.order(tasks::id.desc()).load::<Task>(conn)
    }

    pub fn insert(todo: Todo, conn: &SqliteConnection) -> Result<()> {
        let t = Task { id: None, description: todo.description, completed: false };
        diesel::insert_into(tasks::table).values(&t).execute(conn)?;

        Ok(())
    }

    pub fn toggle_with_id(id: i32, conn: &SqliteConnection) -> Result<()> {
        let task = all_tasks.find(id).get_result::<Task>(conn)?;

        let new_status = !task.completed;
        let updated_task = diesel::update(all_tasks.find(id));
        updated_task.set(task_completed.eq(new_status)).execute(conn)?;

        Ok(())
    }

    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> Result<()> {
        // `.execute` returns a usize representing the number of
        // database rows affected by this modification.
        // In a larger app these metrics may be useful,
        // but for a simple example we'll ignore them.
        diesel::delete(all_tasks.find(id)).execute(conn).map(|_| ())
    }

    #[cfg(test)]
    pub fn delete_all(conn: &SqliteConnection) -> Result<()> {
        // see comment in `.delete_with_id`.
        diesel::delete(all_tasks).execute(conn).map(|_| ())
    }
}
