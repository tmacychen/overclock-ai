//! Overclock Server — REST API for Phase 2 Web Kanban UI.
//! 
//! Provides REST endpoints for task management, agent status,
//! and real-time event streaming (SSE).

use axum::{routing::{get, post}, Router, Json, response::sse::{Event, Sse}, serve};
use overclock_core::event::{EventBus, OrchestratorEvent};
use overclock_core::task::Task;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use futures::stream::{self, Stream};

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskCreate {
    title: String,
    description: String,
    role: String,
}

#[derive(Serialize, Debug)]
pub struct TaskResponse {
    id: String,
    title: String,
    description: String,
    role: String,
    status: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize event bus
    let event_bus = Arc::new(EventBus::default());
    
    // Create task storage
    let tasks: Arc<tokio::sync::RwLock<Vec<Task>>> = Arc::new(tokio::sync::RwLock::new(vec![]));

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Task endpoints
        .route("/tasks", get(get_tasks))
        .route("/tasks", post(create_task))
        .route("/tasks/:id", get(get_task))
        // Event stream
        .route("/events", get(event_stream))
        // Share state
        .with_state((tasks, event_bus));

    // Start server
    println!("Starting overclock-ai server on http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    serve::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

#[axum::debug_handler]
async fn get_tasks(
    axum::extract::State((tasks_lock, _)): axum::extract::State<(Arc<tokio::sync::RwLock<Vec<Task>>>, Arc<EventBus>)>,
) -> Json<Vec<TaskResponse>> {
    let tasks = tasks_lock.read().await;
    
    let response: Vec<TaskResponse> = tasks.iter().map(|task| {
        TaskResponse {
            id: task.id.to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            role: task.role.clone(),
            status: format!("{:?}", task.status),
        }
    }).collect();
    
    Json(response)
}

#[axum::debug_handler]
async fn create_task(
    axum::extract::State((tasks_lock, event_bus)): axum::extract::State<(Arc<tokio::sync::RwLock<Vec<Task>>>, Arc<EventBus>)>,
    Json(task_data): Json<TaskCreate>,
) -> Json<TaskResponse> {
    let task = Task::new(
        &task_data.title,
        &task_data.description,
        &task_data.role,
    );
    
    // Add task to storage
    tasks_lock.write().await.push(task.clone());
    
    // Emit task created event
    event_bus.emit(OrchestratorEvent::TaskCreated {
        task_id: task.id,
        title: task.title.clone(),
        role: task.role.clone(),
    });
    
    let response = TaskResponse {
        id: task.id.to_string(),
        title: task.title,
        description: task.description,
        role: task.role,
        status: format!("{:?}", task.status),
    };
    
    Json(response)
}

#[axum::debug_handler]
async fn get_task(
    axum::extract::State((tasks_lock, _)): axum::extract::State<(Arc<tokio::sync::RwLock<Vec<Task>>>, Arc<EventBus>)>,
    id: axum::extract::Path<String>,
) -> Result<Json<TaskResponse>, axum::http::StatusCode> {
    let tasks = tasks_lock.read().await;
    
    let task = tasks.iter().find(|t| t.id.to_string() == *id);
    match task {
        Some(task) => {
            let response = TaskResponse {
                id: task.id.to_string(),
                title: task.title.clone(),
                description: task.description.clone(),
                role: task.role.clone(),
                status: format!("{:?}", task.status),
            };
            Ok(Json(response))
        }
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

#[axum::debug_handler]
async fn event_stream(
    axum::extract::State((_, event_bus)): axum::extract::State<(Arc<tokio::sync::RwLock<Vec<Task>>>, Arc<EventBus>)>,
) -> Sse<impl Stream<Item = Result<Event, axum::BoxError>>> {
    let receiver = event_bus.subscribe();
    
    let stream = stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(event) => {
                let event_str = format!("{:?}", event);
                Some((
                    Ok(Event::default()
                        .event("message")
                        .data(event_str)),
                    receiver,
                ))
            }
            Err(_) => None,
        }
    });
    
    Sse::new(stream)
}
