use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, PluginError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_sec: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    /// Host attribution for chained calls; never accepted from plugin error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    /// Host-owned display metadata for a failed playback selection, never deserialized from plugins.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub playback_choices: Option<serde_json::Value>,
}

impl PluginError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            retry_after_sec: None,
            retryable: None,
            plugin_id: None,
            playback_choices: None,
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("INVALID_ARGUMENT", message)
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL", message)
    }
    pub fn cancelled() -> Self {
        Self::new("CANCELLED", "调用已取消")
    }
    pub fn timeout() -> Self {
        Self::new("TIMEOUT", "插件调用超时")
    }
}
impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for PluginError {}
impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(format!("无法读写插件数据：{e}"))
    }
}
