use rocket_contrib::databases::mongodb::db::{Database,ThreadedDatabase};
use rocket_contrib::databases::mongodb::{bson,doc,oid::{ObjectId}};

use models::Todo;

const TODOS_COLLECTION: &str = "todos";

#[database("todo_service")]
pub struct MyDatabase(Database);

pub fn get_todos(conn: &MyDatabase) -> Result<Vec<Todo>, &str> {
    let coll = conn.0.collection(TODOS_COLLECTION);
    let cursor = coll.find(None, None).unwrap();
    let mut todos: Vec<Todo> = Vec::new();

    for result in cursor {
        if let Ok(item) = result {
            todos.push(Todo {
                id: item.get_object_id("_id").unwrap().to_string(),
                description: item.get("description").unwrap().to_string(),
            });
        }
    }

    Ok(todos)
}

pub fn get_todo(conn: &MyDatabase, id: String) -> Result<Todo, &str> {

    // MongoDB IDs are ObjectIds not strings
    let object_id = ObjectId::with_string(&id).unwrap();
    let filter = doc!{ "_id": object_id };

    let coll = conn.0.collection(TODOS_COLLECTION);
    match coll.find_one(Some(filter), None).expect("fail") {
        Some(result) => Ok(Todo {
            description: result.get("description").unwrap().to_string(),
            id: result.get_object_id("_id").unwrap().to_string(),
        }),
        None => Err("Todo not found")
    }
}


pub fn insert_todo(conn: &MyDatabase, description: String) -> Result<Todo, &str> {
    let coll = conn.0.collection(TODOS_COLLECTION);
    let new_todo = doc! {
        "description": description.clone(),
    };

    match coll.insert_one(new_todo, None) {
        Ok(result) => Ok(Todo {
            id: result.inserted_id.unwrap().to_string(),
            description: description.clone(),
        }),
        Err(_) => Err("Database error"),
    }
}

pub fn update_todo(conn: &MyDatabase, id: String, description: String) -> Result<(), &str> {
    let coll = conn.0.collection(TODOS_COLLECTION);

    // MongoDB IDs are ObjectIds not strings
    let object_id = ObjectId::with_string(&id).unwrap();
    let filter = doc!{ "_id": object_id };

    // Updates have to include the $set operator
    // https://docs.mongodb.com/manual/reference/operator/update/set/#up._S_set
    let update = doc! {
        "$set": {
            "description": description.clone() 
        }
    };

    match coll.find_one_and_update(filter, update, None) {
        Ok(_) => Ok(()),
        Err(_) => Err("Todo not found")
    }
}

pub fn delete_todo(conn: &MyDatabase, id: String) -> Result<(), &str> {
    let coll = conn.0.collection(TODOS_COLLECTION);

    // MongoDB IDs are ObjectIds not strings
    let object_id = ObjectId::with_string(&id).unwrap();
    let filter = doc!{ "_id": object_id };

    match coll.find_one_and_delete(filter, None) {
        Ok(_) => Ok(()),
        Err(_) => Err("Database error"),
    }
}