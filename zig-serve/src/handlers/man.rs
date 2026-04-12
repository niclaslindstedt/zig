use axum::Json;
use axum::extract::Path;
use serde::Serialize;

use crate::error::ServeError;

#[derive(Serialize)]
pub struct TopicEntry {
    pub topic: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct TopicContent {
    pub topic: String,
    pub content: String,
}

pub async fn list() -> Json<Vec<TopicEntry>> {
    let topics: Vec<TopicEntry> = zig_core::man::TOPICS
        .iter()
        .map(|(topic, description)| TopicEntry {
            topic: (*topic).to_string(),
            description: (*description).to_string(),
        })
        .collect();
    Json(topics)
}

pub async fn show(Path(topic): Path<String>) -> Result<Json<TopicContent>, ServeError> {
    match zig_core::man::get(&topic) {
        Some(content) => Ok(Json(TopicContent {
            topic,
            content: content.to_string(),
        })),
        None => Err(ServeError::not_found(format!(
            "unknown manpage topic: '{topic}'"
        ))),
    }
}
