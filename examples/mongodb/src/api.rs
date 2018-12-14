use rocket_contrib::json::{Json,JsonValue};
use db;

#[get("/")]
pub fn get_todos(conn: db::MyDatabase) -> JsonValue {
    match db::get_todos(&conn) {
        Ok(todos) => json!({
            "status": "success".to_string(),
            "data": todos,
        }),
        Err(e) => json!({
            "status": "error",
            "message": e.to_string(),
        })
    }
}

#[derive(Serialize,Deserialize)]
pub struct CreateTodoRequestBody {
    pub description: String
}

#[get("/<id>")]
pub fn get_todo(conn: db::MyDatabase, id: String) -> JsonValue {
    match db::get_todo(&conn, id.clone()) {
        Ok(todo) => json!({
            "status": "success",
            "data": todo,
        }),
        Err(e) => json!({
            "status": "error",
            "message": e.to_string()
        })
    }
}

#[post("/", format = "json", data = "<body>")]
pub fn create_todo(conn: db::MyDatabase, body: Json<CreateTodoRequestBody>) -> JsonValue {
    match db::insert_todo(&conn, body.description.to_string()) {
        Ok(todo) => json!({
            "status": "success",
            "data": todo
        }),
        Err(e) => json!({
            "status": "error",
            "message": e.to_string()
        })
    }
}

#[derive(Serialize,Deserialize)]
pub struct UpdateTodoRequestBody {
    pub description: String
}

#[patch("/<id>", format = "json", data = "<body>")]
pub fn update_todo(conn: db::MyDatabase, id: String ,body: Json<CreateTodoRequestBody>) -> JsonValue {
    match db::update_todo(&conn, id.clone(), body.description.to_string()) {
        Ok(_) => json!({
            "status": "success",
            "data": ()
        }),
        Err(e) => json!({
            "status": "error",
            "message": e.to_string()
        })
    }
}

#[delete("/<id>")]
pub fn delete_todo(conn: db::MyDatabase, id: String) -> JsonValue {
    match db::delete_todo(&conn, id.clone()) {
        Ok(_) => json!({
            "status": "success",
            "data": ()
        }),
        Err(e) => json!({
            "status": "error",
            "message": e.to_string()
        })
    }
}