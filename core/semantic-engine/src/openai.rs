//! OpenAI Chat Completions API client.
//!
//! Uses OpenAI's tool-calling feature for structured output and plain chat
//! completion for text generation. Exposes `send_structured<T>` and `send_text`
//! using the same `LlmRequest` type as the Anthropic client.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::llm::{LlmError, LlmRequest};

// ============================================================================
// Client
// ============================================================================

#[derive(Debug, Clone)]
pub struct OpenAIClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com".to_string(),
        }
    }

    fn build_headers(&self) -> Result<HeaderMap, LlmError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let auth_value = format!("Bearer {}", self.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).map_err(|e| LlmError::Api {
                status: 0,
                body: format!("Invalid API key header value: {}", e),
            })?,
        );
        Ok(headers)
    }

    async fn send_raw(&self, body: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let headers = self.build_headers()?;

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        let status = resp.status().as_u16();
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Api { status, body });
        }

        let response: ChatResponse = resp.json().await?;
        Ok(response)
    }
}

// ============================================================================
// Public API
// ============================================================================

impl OpenAIClient {
    /// Send a request forcing tool-call output and deserialise as T.
    pub async fn send_structured<T>(&self, req: LlmRequest) -> Result<T, LlmError>
    where
        T: DeserializeOwned + Send,
    {
        let tool = req.tool.ok_or(LlmError::NoStructuredOutput)?;

        // Build messages: OpenAI uses role "system" as a message
        let mut messages: Vec<ChatMessage> = Vec::new();
        if let Some(system) = req.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        for m in req.messages {
            messages.push(ChatMessage {
                role: m.role,
                content: Some(m.content),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let oai_tool = ChatTool {
            tool_type: "function".to_string(),
            function: ChatFunction {
                name: tool.name.clone(),
                description: tool.description,
                parameters: tool.input_schema,
            },
        };

        let body = ChatRequest {
            model: self.model.clone(),
            max_tokens: req.max_tokens,
            messages,
            tools: Some(vec![oai_tool]),
            tool_choice: Some(serde_json::json!({
                "type": "function",
                "function": { "name": tool.name }
            })),
        };

        let response = self.send_raw(&body).await?;

        // Extract tool call arguments
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(LlmError::NoStructuredOutput)?;

        let tool_calls = choice
            .message
            .tool_calls
            .ok_or(LlmError::NoStructuredOutput)?;

        let args_str = tool_calls
            .into_iter()
            .next()
            .ok_or(LlmError::NoStructuredOutput)?
            .function
            .arguments;

        let value: T = serde_json::from_str(&args_str)?;
        Ok(value)
    }

    /// Send a request and return the first text response.
    pub async fn send_text(&self, req: LlmRequest) -> Result<String, LlmError> {
        let mut messages: Vec<ChatMessage> = Vec::new();
        if let Some(system) = req.system {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(system),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        for m in req.messages {
            messages.push(ChatMessage {
                role: m.role,
                content: Some(m.content),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let body = ChatRequest {
            model: self.model.clone(),
            max_tokens: req.max_tokens,
            messages,
            tools: None,
            tool_choice: None,
        };

        let response = self.send_raw(&body).await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(LlmError::NoText)?;

        choice.message.content.ok_or(LlmError::NoText)
    }

    /// The model name in use (for logging).
    pub fn model(&self) -> &str {
        &self.model
    }
}

// ============================================================================
// Request types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ChatTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ChatFunction,
}

#[derive(Debug, Clone, Serialize)]
struct ChatFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serializes_without_tools() {
        let req = ChatRequest {
            model: "gpt-4o".to_string(),
            max_tokens: 1024,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            tool_choice: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(!json.as_object().unwrap().contains_key("tools"));
        assert!(!json.as_object().unwrap().contains_key("tool_choice"));
        assert_eq!(json["model"], "gpt-4o");
    }

    #[test]
    fn chat_request_serializes_with_tools() {
        let req = ChatRequest {
            model: "gpt-4o".to_string(),
            max_tokens: 1024,
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: Some("You are a parser.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: Some(vec![ChatTool {
                tool_type: "function".to_string(),
                function: ChatFunction {
                    name: "parse_intent".to_string(),
                    description: "Parse intent".to_string(),
                    parameters: serde_json::json!({"type": "object"}),
                },
            }]),
            tool_choice: Some(serde_json::json!({"type": "function", "function": {"name": "parse_intent"}})),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["tools"][0]["function"]["name"], "parse_intent");
        assert_eq!(json["tool_choice"]["type"], "function");
    }

    #[test]
    fn chat_response_with_tool_call_deserializes() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "function": {
                            "name": "parse_intent",
                            "arguments": "{\"decision_class\": \"churn_intervention\"}"
                        }
                    }]
                }
            }]
        });
        let resp: ChatResponse = serde_json::from_value(json).unwrap();
        let tool_calls = resp.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls[0].function.name, "parse_intent");
        assert!(tool_calls[0].function.arguments.contains("churn_intervention"));
    }

    #[test]
    fn chat_response_text_deserializes() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Here is the analysis."
                }
            }]
        });
        let resp: ChatResponse = serde_json::from_value(json).unwrap();
        assert_eq!(
            resp.choices[0].message.content.as_deref(),
            Some("Here is the analysis.")
        );
    }
}
