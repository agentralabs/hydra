use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use hydra_db::{Conversation, DbError, Message, MessageRole};

use crate::state::AppState;

/// Route definitions for conversation and message management.
pub struct MessageRoutes;

impl MessageRoutes {
    /// GET: list all conversations
    pub fn list_conversations() -> &'static str {
        "/api/conversations"
    }

    /// POST: create a new conversation
    pub fn create_conversation() -> &'static str {
        "/api/conversations"
    }

    /// GET: retrieve a specific conversation by ID
    pub fn get_conversation() -> &'static str {
        "/api/conversations/:id"
    }

    /// DELETE: delete a conversation
    pub fn delete_conversation() -> &'static str {
        "/api/conversations/:id"
    }

    /// GET: list messages in a conversation
    pub fn list_messages() -> &'static str {
        "/api/conversations/:id/messages"
    }

    /// POST: send a message to a conversation
    pub fn send_message() -> &'static str {
        "/api/conversations/:id/messages"
    }

    /// GET: retrieve a specific message
    pub fn get_message() -> &'static str {
        "/api/conversations/:id/messages/:msg_id"
    }

    /// POST: retry the last message in a conversation
    pub fn retry_message() -> &'static str {
        "/api/conversations/:id/retry"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub role: Option<MessageRole>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationWithMessages {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct DeletedResponse {
    pub deleted: bool,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

fn map_db_err(e: DbError) -> (StatusCode, String) {
    match &e {
        DbError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// GET /api/conversations — list all conversations
pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Conversation>>, (StatusCode, String)> {
    let convos = state.message_store.list_conversations().map_err(map_db_err)?;
    Ok(Json(convos))
}

/// POST /api/conversations — create a new conversation
pub async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<Conversation>), (StatusCode, String)> {
    let now = Utc::now().to_rfc3339();
    let conv = Conversation {
        id: Uuid::new_v4().to_string(),
        title: req.title,
        created_at: now.clone(),
        updated_at: now,
    };
    state
        .message_store
        .create_conversation(&conv)
        .map_err(map_db_err)?;
    Ok((StatusCode::CREATED, Json(conv)))
}

/// GET /api/conversations/:id — get conversation with its messages
pub async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConversationWithMessages>, (StatusCode, String)> {
    let conversation = state
        .message_store
        .get_conversation_info(&id)
        .map_err(map_db_err)?;
    let messages = state
        .message_store
        .get_conversation(&id, None)
        .map_err(map_db_err)?;
    Ok(Json(ConversationWithMessages {
        conversation,
        messages,
    }))
}

/// DELETE /api/conversations/:id — delete a conversation
pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeletedResponse>, (StatusCode, String)> {
    state
        .message_store
        .delete_conversation(&id)
        .map_err(map_db_err)?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// POST /api/conversations/:id/messages — send a message
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), (StatusCode, String)> {
    // Verify conversation exists
    state
        .message_store
        .get_conversation_info(&id)
        .map_err(map_db_err)?;

    let msg = Message {
        id: Uuid::new_v4().to_string(),
        conversation_id: id,
        role: req.role.unwrap_or(MessageRole::User),
        content: req.content,
        created_at: Utc::now().to_rfc3339(),
        run_id: None,
        metadata: req.metadata,
    };
    state.message_store.add_message(&msg).map_err(map_db_err)?;
    Ok((StatusCode::CREATED, Json(msg)))
}

/// GET /api/conversations/:id/messages — alias; same data as get_conversation messages
pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Message>>, (StatusCode, String)> {
    let messages = state
        .message_store
        .get_conversation(&id, None)
        .map_err(map_db_err)?;
    Ok(Json(messages))
}

/// GET /api/search?q=... — full-text search across messages
pub async fn search_messages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<Message>>, (StatusCode, String)> {
    let results = state
        .message_store
        .search(&params.q)
        .map_err(map_db_err)?;
    Ok(Json(results))
}

/// GET /api/conversations/:id/messages/:msg_id — get a single message
pub async fn get_message(
    State(state): State<Arc<AppState>>,
    Path((id, msg_id)): Path<(String, String)>,
) -> Result<Json<Message>, (StatusCode, String)> {
    let msg = state
        .message_store
        .get_message(&id, &msg_id)
        .map_err(map_db_err)?;
    Ok(Json(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_conversations_path() {
        assert_eq!(MessageRoutes::list_conversations(), "/api/conversations");
    }

    #[test]
    fn test_create_conversation_path() {
        assert_eq!(MessageRoutes::create_conversation(), "/api/conversations");
    }

    #[test]
    fn test_get_conversation_path() {
        assert_eq!(MessageRoutes::get_conversation(), "/api/conversations/:id");
    }

    #[test]
    fn test_delete_conversation_path() {
        assert_eq!(MessageRoutes::delete_conversation(), "/api/conversations/:id");
    }

    #[test]
    fn test_list_messages_path() {
        assert_eq!(MessageRoutes::list_messages(), "/api/conversations/:id/messages");
    }

    #[test]
    fn test_send_message_path() {
        assert_eq!(MessageRoutes::send_message(), "/api/conversations/:id/messages");
    }

    #[test]
    fn test_get_message_path() {
        assert_eq!(MessageRoutes::get_message(), "/api/conversations/:id/messages/:msg_id");
    }

    #[test]
    fn test_retry_message_path() {
        assert_eq!(MessageRoutes::retry_message(), "/api/conversations/:id/retry");
    }

    #[test]
    fn test_create_conversation_request_deserialization() {
        let json = serde_json::json!({"title": "My Conversation"});
        let req: CreateConversationRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.title, Some("My Conversation".into()));
    }

    #[test]
    fn test_create_conversation_request_no_title() {
        let json = serde_json::json!({});
        let req: CreateConversationRequest = serde_json::from_value(json).unwrap();
        assert!(req.title.is_none());
    }

    #[test]
    fn test_send_message_request_deserialization() {
        let json = serde_json::json!({"content": "Hello!", "role": "user"});
        let req: SendMessageRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.content, "Hello!");
    }

    #[test]
    fn test_send_message_request_minimal() {
        let json = serde_json::json!({"content": "Hello"});
        let req: SendMessageRequest = serde_json::from_value(json).unwrap();
        assert!(req.role.is_none());
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_search_query_deserialization() {
        let json = serde_json::json!({"q": "test query"});
        let q: SearchQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.q, "test query");
    }

    #[test]
    fn test_deleted_response_serialization() {
        let resp = DeletedResponse { deleted: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["deleted"], true);
    }
}

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
