//! Priority 6: Deep Comm Integration — agent-to-agent messaging,
//! swarm coordination, message routing through Comm sister.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    /// Register an agent with Comm sister for messaging.
    pub async fn comm_register_agent(
        &self,
        agent_id: &str,
        role: &str,
    ) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_register_agent", serde_json::json!({
            "agent_id": agent_id,
            "role": role,
            "capabilities": ["task_execution", "status_report"],
        })).await.ok()?;
        result.get("channel_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Send a message from one agent to another via Comm sister.
    /// Comm handles consent verification, rate limiting, and audit.
    pub async fn comm_send_message(
        &self,
        from: &str,
        to: &str,
        message_type: &str,
        content: &str,
    ) -> bool {
        if let Some(comm) = &self.comm {
            let result = comm.call_tool("comm_send", serde_json::json!({
                "from": from,
                "to": to,
                "message_type": message_type,
                "content": safe_truncate(content, 500),
            })).await;
            result.is_ok()
        } else {
            false
        }
    }

    /// Broadcast a message to all agents in the swarm.
    pub async fn comm_broadcast(
        &self,
        from: &str,
        message_type: &str,
        content: &str,
    ) -> bool {
        if let Some(comm) = &self.comm {
            let result = comm.call_tool("comm_broadcast", serde_json::json!({
                "from": from,
                "message_type": message_type,
                "content": safe_truncate(content, 500),
                "scope": "swarm",
            })).await;
            result.is_ok()
        } else {
            false
        }
    }

    /// Check for pending messages/notifications for an agent.
    pub async fn comm_check_inbox(&self, agent_id: &str, limit: usize) -> Option<Vec<CommMessage>> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_inbox", serde_json::json!({
            "agent_id": agent_id,
            "limit": limit,
        })).await.ok()?;

        let messages: Vec<CommMessage> = result.get("messages")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|m| {
                let from = m.get("from")?.as_str()?.to_string();
                let content = m.get("content")?.as_str()?.to_string();
                let msg_type = m.get("message_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                Some(CommMessage { from, content, message_type: msg_type })
            }).collect())
            .unwrap_or_default();

        if messages.is_empty() { None } else { Some(messages) }
    }

    /// Deregister an agent when it's terminated.
    pub async fn comm_deregister_agent(&self, agent_id: &str) {
        if let Some(comm) = &self.comm {
            let _ = comm.call_tool("comm_deregister_agent", serde_json::json!({
                "agent_id": agent_id,
            })).await;
        }
    }

    /// Get communication health metrics for the swarm.
    pub async fn comm_swarm_health(&self) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_health", serde_json::json!({
            "scope": "swarm",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// SESSION: Start a communication session for tracking this conversation.
    /// Returns a session_id used for subsequent log/context calls.
    pub async fn comm_session_start(&self, user_name: &str) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_session", serde_json::json!({
            "operation": "start",
            "params": {
                "agent_id": user_name,
                "session_type": "conversation",
            }
        })).await.ok()?;
        result.get("session_id").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// SESSION: Log an interaction within the communication session.
    /// Creates a durable conversation trail for context continuity.
    pub async fn comm_session_log(
        &self,
        user_msg: &str,
        response: &str,
    ) {
        if let Some(comm) = &self.comm {
            let _ = comm.call_tool("comm_session", serde_json::json!({
                "operation": "log",
                "params": {
                    "user_message": safe_truncate(user_msg, 300),
                    "assistant_response": safe_truncate(response, 300),
                }
            })).await;
        }
    }

    /// SESSION: Get conversation context from the communication session.
    /// Returns accumulated context from previous exchanges in this session.
    pub async fn comm_session_context(&self) -> Option<String> {
        let comm = self.comm.as_ref()?;
        let result = comm.call_tool("comm_session", serde_json::json!({
            "operation": "get_context",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// SESSION: End the communication session with a summary.
    pub async fn comm_session_end(&self, summary: &str) {
        if let Some(comm) = &self.comm {
            let _ = comm.call_tool("comm_session", serde_json::json!({
                "operation": "end",
                "params": {
                    "summary": safe_truncate(summary, 300),
                }
            })).await;
        }
    }
}

/// A message from the Comm sister inbox.
#[derive(Debug, Clone)]
pub struct CommMessage {
    pub from: String,
    pub content: String,
    pub message_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comm_message_struct() {
        let m = CommMessage {
            from: "agent-001".into(),
            content: "Task completed".into(),
            message_type: "status_update".into(),
        };
        assert_eq!(m.from, "agent-001");
        assert_eq!(m.message_type, "status_update");
    }
}
