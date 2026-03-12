use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use hydra_db::{Message, MessageRole};

use crate::state::AppState;

use super::messages::map_db_err;

/// POST /api/conversations/:id/retry — retry the last message in a conversation
pub async fn retry_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Message>, (StatusCode, String)> {
    // Get all messages to find the last user message
    let messages = state
        .message_store
        .get_conversation(&id, None)
        .map_err(map_db_err)?;

    let last_user_msg = messages
        .iter()
        .rev()
        .find(|m| m.role == MessageRole::User)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No user message found in conversation {id}"),
            )
        })?;

    // Re-send the last user message as a new message
    let retry_msg = Message {
        id: Uuid::new_v4().to_string(),
        conversation_id: id,
        role: MessageRole::User,
        content: last_user_msg.content.clone(),
        created_at: Utc::now().to_rfc3339(),
        run_id: None,
        metadata: Some(serde_json::json!({
            "retry": true,
            "original_message_id": last_user_msg.id,
        })),
    };

    state
        .message_store
        .add_message(&retry_msg)
        .map_err(map_db_err)?;

    Ok(Json(retry_msg))
}
