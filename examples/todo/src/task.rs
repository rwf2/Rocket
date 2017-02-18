use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use self::schema::tasks;
use self::schema::tasks::dsl::{tasks as all_tasks, completed as task_completed};

mod schema {
    infer_schema!("env:DATABASE_URL");
}

#[table_name = "tasks"]
#[derive(Serialize, Queryable, Insertable, FromForm, Debug, Clone)]
pub struct Task {
    id: Option<i32>,
    pub description: String,
    pub completed: Option<bool>
}

impl Task {
    pub fn all(conn: &SqliteConnection) -> Vec<Task> {
        all_tasks.order(tasks::id.desc()).load::<Task>(conn).unwrap()
    }

    pub fn insert(&self, conn: &SqliteConnection) -> bool {
        diesel::insert(self).into(tasks::table).execute(conn).is_ok()
    }

    pub fn toggle_with_id(id: i32, conn: &SqliteConnection) -> bool {
        let task = all_tasks.find(id).get_result::<Task>(conn);
        if task.is_err() {
            return false;
        }

        let new_status = !task.unwrap().completed.unwrap();
        let updated_task = diesel::update(all_tasks.find(id));
        updated_task.set(task_completed.eq(new_status)).execute(conn).is_ok()
    }

    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks.find(id)).execute(conn).is_ok()
    }
}

