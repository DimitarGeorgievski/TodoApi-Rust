use axum::{Json, Router, http::StatusCode, routing::{get, post}};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::SocketAddr;
use std::env;
use validator::Validate;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    let app = Router::new()
        .route("/", get(root))
        .route("/todos", get(get_todos).post(create_todo))
        .layer(axum::extract::Extension(pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello this is my Todo App"
}

#[derive(Serialize, Deserialize)]
struct Todo {
    id: i32,
    title: String,
    completed: bool,
}

async fn get_todos(
    axum::extract::Extension(pool): axum::extract::Extension<PgPool>
) -> Json<Vec<Todo>> {
    let todos = sqlx::query_as!(
        Todo,
        r#"SELECT id, title, completed FROM todos ORDER BY id"#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Json(todos)
}

#[derive(Deserialize, Validate)]
struct CreateTodo {
    #[validate(
        length(min = 3, max = 50, message = "Title must be between 3 and 50 characters"),
    )]
    title: String,
}
async fn create_todo(
    axum::extract::Extension(pool): axum::extract::Extension<PgPool>,
    Json(payload): Json<CreateTodo>,
) -> Result<Json<Todo>, (StatusCode, String)> {
   if let Err(e) = payload.validate() {
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }
    let found = sqlx::query!(
        r#"SELECT id FROM todos WHERE title = $1"#,
        payload.title
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    if found.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Todo with this title already exists".into(),
        ));
    }
    let todo = sqlx::query_as!(
        Todo,
        r#"INSERT INTO todos (title, completed)
           VALUES ($1, false)
           RETURNING id, title, completed"#,
        payload.title
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    Ok(Json(todo))
}
