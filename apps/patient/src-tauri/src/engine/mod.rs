pub mod boa;

/// Response from the embedded JS engine
#[derive(Debug, serde::Serialize)]
pub struct EngineResponse {
    pub status: u16,
    pub body: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<(String, String)>,
    /// JS console output captured during this request
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
}

/// Trait for JS engine implementations
pub trait JsEngine: Send + Sync {
    fn handle_request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
        headers: Vec<(&str, &str)>,
    ) -> Result<EngineResponse, String>;
}
