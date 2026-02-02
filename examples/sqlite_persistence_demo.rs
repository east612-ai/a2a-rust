//! SQLite Persistence Demo
//! 
//! This example demonstrates how to use the SQLite-based task store
//! for persistent storage of A2A tasks.

use a2a_rust::a2a::{
    models::*,
    core_types::*,
    server::tasks::{SqliteTaskStore, TaskStore, TaskManager},
};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 A2A SQLite Persistence Demo");
    println!("{}", "=".repeat(40));

    // 1. Initialize SQLite Task Store
    // We use an in-memory SQLite database for this demo, 
    // but you can use a file path like "sqlite://a2a_tasks.db"
    let db_url = "sqlite::memory:";
    println!("🔗 Connecting to database: {}", db_url);
    
    let task_store = SqliteTaskStore::connect(db_url).await?;
    task_store.initialize().await?;
    println!("✅ Database initialized successfully.");

    let task_store = Arc::new(task_store);

    // 2. Create a TaskManager with the SQLite store
    let mut task_manager = TaskManager::new(
        None, // task_id
        None, // context_id
        task_store.clone(),
        None, // initial_message
        None, // context placeholder
    )?;

    // 3. Create and save a new task
    let task_id = "task-demo-123";
    let context_id = "ctx-demo-456";
    
    println!("\n📝 Creating a new task: {}", task_id);
    let mut task = Task::new(
        context_id.to_string(),
        TaskStatus::new(TaskState::Submitted),
    ).with_task_id(task_id.to_string());
    
    // Add a message to the task history
    let message = Message::new(
        Role::User,
        vec![Part::text("Hello, please help me with persistence!".to_string())]
    );
    task = task_manager.update_with_message(message, task).await;
    
    // Save the task to SQLite
    task_store.save(task.clone()).await?;
    println!("✅ Task saved to SQLite.");

    // 4. Retrieve the task from SQLite
    println!("\n🔍 Retrieving task from database...");
    let retrieved_task = task_store.get(task_id).await?;
    
    if let Some(t) = retrieved_task {
        println!("✅ Found task: {}", t.id);
        println!("   Status: {:?}", t.status.state);
        if let Some(history) = &t.history {
            println!("   History length: {}", history.len());
        }
    } else {
        println!("❌ Task not found!");
    }

    // 5. List all tasks
    println!("\n📋 Listing all tasks in database:");
    let all_tasks = task_store.list().await?;
    for t in all_tasks {
        println!("   - ID: {}, Context: {}, State: {:?}", t.id, t.context_id, t.status.state);
    }

    println!("\n🎯 Demo completed successfully!");
    Ok(())
}
