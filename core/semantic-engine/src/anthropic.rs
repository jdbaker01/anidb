//! Anthropic Messages API client using reqwest directly.
//!
//! This resolves OQ-001: we use reqwest with typed request/response structs
//! instead of a third-party SDK. The API surface is small (two call patterns:
//! structured output via tool_use, and text generation) and this gives full
//! control without external SDK dependencies.

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// ============================================================================
// Client
// ============================================================================

#[derive(Debug, Clone)]
pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    /// Returns the configured model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Send a messages request and return the full response.
    pub async fn send(&self, request: MessageRequest) -> Result<MessageResponse, AnthropicError> {
        let url = format!("{}/v1/messages", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| AnthropicError::Api {
                    status: 0,
                    body: format!("Invalid API key header value: {}", e),
                })?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(AnthropicError::Api { status, body });
        }

        let response: MessageResponse = resp.json().await?;
        Ok(response)
    }

    /// Send a request that forces a tool_use response, then deserialize
    /// the tool input as type T.
    pub async fn send_structured<T: DeserializeOwned>(
        &self,
        request: MessageRequest,
    ) -> Result<T, AnthropicError> {
        let response = self.send(request).await?;

        // Find the tool_use content block
        for block in &response.content {
            if let ContentBlock::ToolUse { input, .. } = block {
                let value: T = serde_json::from_value(input.clone())?;
                return Ok(value);
            }
        }

        Err(AnthropicError::NoToolUse)
    }

    /// Send a request and extract the first text block.
    pub async fn send_text(&self, request: MessageRequest) -> Result<String, AnthropicError> {
        let response = self.send(request).await?;

        for block in &response.content {
            if let ContentBlock::Text { text } = block {
                return Ok(text.clone());
            }
        }

        Err(AnthropicError::NoText)
    }
}

// ============================================================================
// Request types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
    pub name: String,
}

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API returned error {status}: {body}")]
    Api { status: u16, body: String },

    #[error("Failed to deserialize response: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("No tool_use block in response")]
    NoToolUse,

    #[error("No text block in response")]
    NoText,
}

impl std::fmt::Display for AnthropicClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnthropicClient(model={}, base_url={})", self.model, self.base_url)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_request_serializes_without_optional_fields() {
        let req = MessageRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            system: None,
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
            }],
            tools: None,
            tool_choice: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(!json.as_object().unwrap().contains_key("system"));
        assert!(!json.as_object().unwrap().contains_key("tools"));
        assert!(!json.as_object().unwrap().contains_key("tool_choice"));
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
    }

    #[test]
    fn message_request_serializes_with_tools() {
        let req = MessageRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            system: Some("You are a parser.".to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Parse this intent".to_string()),
            }],
            tools: Some(vec![ToolDef {
                name: "parse_intent".to_string(),
                description: "Parse intent".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }]),
            tool_choice: Some(ToolChoice {
                choice_type: "tool".to_string(),
                name: "parse_intent".to_string(),
            }),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["system"], "You are a parser.");
        assert_eq!(json["tools"][0]["name"], "parse_intent");
        assert_eq!(json["tool_choice"]["type"], "tool");
    }

    #[test]
    fn content_block_text_roundtrip() {
        let block = ContentBlock::Text {
            text: "Hello world".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        match parsed {
            ContentBlock::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected Text block"),
        }
    }

    #[test]
    fn content_block_tool_use_roundtrip() {
        let block = ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "parse_intent".to_string(),
            input: serde_json::json!({"decision_class": "churn_intervention"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        match parsed {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_123");
                assert_eq!(name, "parse_intent");
                assert_eq!(input["decision_class"], "churn_intervention");
            }
            _ => panic!("Expected ToolUse block"),
        }
    }

    #[test]
    fn message_response_deserializes() {
        let json = serde_json::json!({
            "id": "msg_123",
            "content": [
                {"type": "text", "text": "Here is the analysis."}
            ],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 100, "output_tokens": 50}
        });
        let resp: MessageResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.id, "msg_123");
        assert_eq!(resp.content.len(), 1);
        assert_eq!(resp.usage.input_tokens, 100);
    }

    #[test]
    fn message_response_with_tool_use_deserializes() {
        let json = serde_json::json!({
            "id": "msg_456",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_789",
                    "name": "parse_intent",
                    "input": {
                        "decision_class": "churn_intervention",
                        "entity_refs": [],
                        "time_horizon": {"lookback_days": 30, "forecast_days": 30},
                        "min_confidence": 0.5,
                        "required_data": [],
                        "interpretation": "Find churning customers"
                    }
                }
            ],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 200, "output_tokens": 100}
        });
        let resp: MessageResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.stop_reason, Some("tool_use".to_string()));
        match &resp.content[0] {
            ContentBlock::ToolUse { name, input, .. } => {
                assert_eq!(name, "parse_intent");
                assert_eq!(input["decision_class"], "churn_intervention");
            }
            _ => panic!("Expected ToolUse block"),
        }
    }
}
