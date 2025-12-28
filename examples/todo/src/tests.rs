use super::task::Task;

use rand::{self, Rng, distr::Alphanumeric};

use rocket::local::asynchronous::Client;
use rocket::http::{Status, ContentType};

// We use a lock to synchronize between tests so DB operations don't collide.
// For now. In the future, we'll have a nice way to run each test in a DB
// transaction so we can regain concurrency.
static DB_LOCK: parking_lot::Mutex<()> = parking_lot::const_mutex(());

macro_rules! run_test {
    (|$client:ident, $conn:ident| $block:expr) => ({
        let _lock = DB_LOCK.lock();

        rocket::async_test(async move {
            let $client = Client::tracked(super::rocket()).await.expect("Rocket client");
            let db = super::DbConn::get_one($client.rocket()).await;
            let $conn = db.expect("failed to get database connection for testing");
            Task::delete_all(&$conn).await.expect("failed to delete all tasks for testing");

            $block
        })
    })
}

#[test]
fn test_index() {
    use rocket::local::blocking::Client;

    let _lock = DB_LOCK.lock();
    let client = Client::tracked(super::rocket()).unwrap();
    let response = client.get("/").dispatch();
    assert_eq!(response.status(), Status::Ok);
}

#[test]
fn test_insertion_deletion() {
    run_test!(|client, conn| {
        // Get the tasks before making changes.
        let init_tasks = Task::all(&conn).await.unwrap();

        // Issue a request to insert a new task.
        client.post("/todo")
            .header(ContentType::Form)
            .body("description=My+first+task")
            .dispatch()
            .await;

        // Ensure we have one more task in the database.
        let new_tasks = Task::all(&conn).await.unwrap();
        assert_eq!(new_tasks.len(), init_tasks.len() + 1);

        // Ensure the task is what we expect.
        assert_eq!(new_tasks[0].description, "My first task");
        assert!(!new_tasks[0].completed);

        // Issue a request to delete the task.
        let id = new_tasks[0].id.unwrap();
        client.delete(format!("/todo/{}", id)).dispatch().await;

        // Ensure it's gone.
        let final_tasks = Task::all(&conn).await.unwrap();
        assert_eq!(final_tasks.len(), init_tasks.len());
        if !final_tasks.is_empty() {
            assert_ne!(final_tasks[0].description, "My first task");
        }
    })
}

#[test]
fn test_toggle() {
    run_test!(|client, conn| {
        // Issue a request to insert a new task; ensure it's not yet completed.
        client.post("/todo")
            .header(ContentType::Form)
            .body("description=test_for_completion")
            .dispatch()
            .await;

        let task = Task::all(&conn).await.unwrap()[0].clone();
        assert!(!task.completed);

        // Issue a request to toggle the task; ensure it is completed.
        client.put(format!("/todo/{}", task.id.unwrap())).dispatch().await;
        assert!(Task::all(&conn).await.unwrap()[0].completed);

        // Issue a request to toggle the task; ensure it's not completed again.
        client.put(format!("/todo/{}", task.id.unwrap())).dispatch().await;
        assert!(!Task::all(&conn).await.unwrap()[0].completed);
    })
}

#[test]
fn test_many_insertions() {
    const ITER: usize = 100;

    run_test!(|client, conn| {
        // Get the number of tasks initially.
        let init_num = Task::all(&conn).await.unwrap().len();
        let mut descs = Vec::new();

        for i in 0..ITER {
            // Issue a request to insert a new task with a random description.
            let desc: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(12)
                .map(char::from)
                .collect();

            client.post("/todo")
                .header(ContentType::Form)
                .body(format!("description={}", desc))
                .dispatch()
                .await;

            // Record the description we choose for this iteration.
            descs.insert(0, desc);

            // Ensure the task was inserted properly and all other tasks remain.
            let tasks = Task::all(&conn).await.unwrap();
            assert_eq!(tasks.len(), init_num + i + 1);

            for j in 0..i {
                assert_eq!(descs[j], tasks[j].description);
            }
        }
    })
}

#[test]
fn test_bad_form_submissions() {
    run_test!(|client, _conn| {
        // Submit an empty form. We should get a 422 but no flash error.
        let res = client.post("/todo")
            .header(ContentType::Form)
            .dispatch()
            .await;

        assert!(!res.cookies().iter().any(|c| c.value().contains("error")));
        assert_eq!(res.status(), Status::UnprocessableEntity);

        // Submit a form with an empty description. We look for 'error' in the
        // cookies which corresponds to flash message being set as an error.
        let res = client.post("/todo")
            .header(ContentType::Form)
            .body("description=")
            .dispatch()
            .await;

        // Check that the flash cookie set and that we're redirected to index.
        assert!(res.cookies().iter().any(|c| c.value().contains("error")));
        assert_eq!(res.status(), Status::SeeOther);

        // The flash cookie should still be present and the error message should
        // be rendered the index.
        let body = client.get("/").dispatch().await.into_string().await.unwrap();
        assert!(body.contains("Description cannot be empty."));

        // Check that the flash is cleared upon another visit to the index.
        let body = client.get("/").dispatch().await.into_string().await.unwrap();
        assert!(!body.contains("Description cannot be empty."));

        // Submit a form without a description. Expect a 422 but no flash error.
        let res = client.post("/todo")
            .header(ContentType::Form)
            .body("evil=smile")
            .dispatch()
            .await;

        assert!(!res.cookies().iter().any(|c| c.value().contains("error")));
        assert_eq!(res.status(), Status::UnprocessableEntity);
    })
}
