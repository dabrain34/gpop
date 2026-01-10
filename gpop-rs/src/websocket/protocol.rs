use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::PipelineState;

/// JSON-RPC 2.0 standard error codes
pub mod error_codes {
    /// Parse error - Invalid JSON was received
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid Request - The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found - The method does not exist / is not available
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params - Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error - Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;

    // Server error codes (reserved for implementation-defined server errors)
    // Range: -32000 to -32099

    /// Pipeline not found
    pub const PIPELINE_NOT_FOUND: i32 = -32000;
    /// Pipeline creation failed
    pub const PIPELINE_CREATION_FAILED: i32 = -32001;
    /// State change failed
    pub const STATE_CHANGE_FAILED: i32 = -32002;
    /// GStreamer error
    pub const GSTREAMER_ERROR: i32 = -32003;
    /// Pipeline description too long
    pub const DESCRIPTION_TOO_LONG: i32 = -32004;
}

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub code: i32,
    pub message: String,
}

impl Response {
    pub fn success(id: String, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: String, code: i32, message: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(ErrorInfo { code, message }),
        }
    }

    /// Create a parse error response
    pub fn parse_error(id: String, message: String) -> Self {
        Self::error(id, error_codes::PARSE_ERROR, message)
    }

    /// Create a method not found error response
    pub fn method_not_found(id: String, method: &str) -> Self {
        Self::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", method),
        )
    }

    /// Create an invalid params error response
    pub fn invalid_params(id: String, message: String) -> Self {
        Self::error(id, error_codes::INVALID_PARAMS, message)
    }

    /// Create a pipeline not found error response
    pub fn pipeline_not_found(id: String, pipeline_id: &str) -> Self {
        Self::error(
            id,
            error_codes::PIPELINE_NOT_FOUND,
            format!("Pipeline not found: {}", pipeline_id),
        )
    }

    /// Create a server error response from a GpopError
    pub fn from_gpop_error(id: String, err: &crate::error::GpopError) -> Self {
        use crate::error::GpopError;

        let (code, message) = match err {
            GpopError::PipelineNotFound(pid) => {
                (error_codes::PIPELINE_NOT_FOUND, format!("Pipeline not found: {}", pid))
            }
            GpopError::InvalidPipeline(msg) => {
                (error_codes::PIPELINE_CREATION_FAILED, msg.clone())
            }
            GpopError::StateChangeFailed(msg) => {
                (error_codes::STATE_CHANGE_FAILED, msg.clone())
            }
            GpopError::GStreamer(msg) => {
                (error_codes::GSTREAMER_ERROR, msg.clone())
            }
            _ => (error_codes::INTERNAL_ERROR, err.to_string()),
        };

        Self::error(id, code, message)
    }
}

// Request parameter types
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePipelineParams {
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipelineIdParams {
    pub pipeline_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetStateParams {
    pub pipeline_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetDotParams {
    pub pipeline_id: String,
    #[serde(default)]
    pub details: Option<String>,
}

// Response result types
#[derive(Debug, Clone, Serialize)]
pub struct PipelineCreatedResult {
    pub pipeline_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineInfoResult {
    pub id: String,
    pub description: String,
    pub state: PipelineState,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListPipelinesResult {
    pub pipelines: Vec<PipelineInfoResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuccessResult {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DotResult {
    pub dot: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_deserialize() {
        let json = r#"{"id":"123","method":"list_pipelines","params":{}}"#;
        let request: Request = serde_json::from_str(json).unwrap();

        assert_eq!(request.id, "123");
        assert_eq!(request.method, "list_pipelines");
    }

    #[test]
    fn test_request_deserialize_with_params() {
        let json = r#"{"id":"456","method":"create_pipeline","params":{"description":"videotestsrc ! fakesink"}}"#;
        let request: Request = serde_json::from_str(json).unwrap();

        assert_eq!(request.id, "456");
        assert_eq!(request.method, "create_pipeline");

        let params: CreatePipelineParams = serde_json::from_value(request.params).unwrap();
        assert_eq!(params.description, "videotestsrc ! fakesink");
    }

    #[test]
    fn test_request_deserialize_optional_params() {
        let json = r#"{"id":"789","method":"list_pipelines"}"#;
        let request: Request = serde_json::from_str(json).unwrap();

        assert_eq!(request.id, "789");
        assert_eq!(request.method, "list_pipelines");
        assert!(request.params.is_null());
    }

    #[test]
    fn test_response_success() {
        let response = Response::success(
            "123".to_string(),
            serde_json::json!({"pipeline_id": "pipeline-0"}),
        );

        assert_eq!(response.id, "123");
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"pipeline_id\":\"pipeline-0\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error() {
        let response = Response::error("123".to_string(), -32600, "Invalid request".to_string());

        assert_eq!(response.id, "123");
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid request");
    }

    #[test]
    fn test_response_serialization_skips_none() {
        let success = Response::success("1".to_string(), serde_json::json!({}));
        let json = serde_json::to_string(&success).unwrap();
        assert!(!json.contains("\"error\""));

        let error = Response::error("2".to_string(), error_codes::INTERNAL_ERROR, "Error".to_string());
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_create_pipeline_params() {
        let json = r#"{"description":"videotestsrc ! fakesink"}"#;
        let params: CreatePipelineParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.description, "videotestsrc ! fakesink");
    }

    #[test]
    fn test_pipeline_id_params() {
        let json = r#"{"pipeline_id":"pipeline-0"}"#;
        let params: PipelineIdParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.pipeline_id, "pipeline-0");
    }

    #[test]
    fn test_set_state_params() {
        let json = r#"{"pipeline_id":"pipeline-0","state":"playing"}"#;
        let params: SetStateParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.pipeline_id, "pipeline-0");
        assert_eq!(params.state, "playing");
    }

    #[test]
    fn test_get_dot_params_with_details() {
        let json = r#"{"pipeline_id":"pipeline-0","details":"all"}"#;
        let params: GetDotParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.pipeline_id, "pipeline-0");
        assert_eq!(params.details, Some("all".to_string()));
    }

    #[test]
    fn test_get_dot_params_without_details() {
        let json = r#"{"pipeline_id":"pipeline-0"}"#;
        let params: GetDotParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.pipeline_id, "pipeline-0");
        assert!(params.details.is_none());
    }

    #[test]
    fn test_pipeline_info_result() {
        let result = PipelineInfoResult {
            id: "pipeline-0".to_string(),
            description: "videotestsrc ! fakesink".to_string(),
            state: PipelineState::Playing,
            streaming: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"id\":\"pipeline-0\""));
        assert!(json.contains("\"state\":\"playing\""));
        assert!(json.contains("\"streaming\":true"));
    }

    #[test]
    fn test_list_pipelines_result() {
        let result = ListPipelinesResult {
            pipelines: vec![
                PipelineInfoResult {
                    id: "pipeline-0".to_string(),
                    description: "test1".to_string(),
                    state: PipelineState::Null,
                    streaming: false,
                },
                PipelineInfoResult {
                    id: "pipeline-1".to_string(),
                    description: "test2".to_string(),
                    state: PipelineState::Playing,
                    streaming: true,
                },
            ],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"pipelines\":["));
        assert!(json.contains("\"pipeline-0\""));
        assert!(json.contains("\"pipeline-1\""));
    }

    #[test]
    fn test_success_result() {
        let result = SuccessResult { success: true };
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, r#"{"success":true}"#);
    }

    #[test]
    fn test_dot_result() {
        let result = DotResult {
            dot: "digraph pipeline {}".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("digraph pipeline"));
    }

    #[test]
    fn test_pipeline_created_result() {
        let result = PipelineCreatedResult {
            pipeline_id: "pipeline-0".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, r#"{"pipeline_id":"pipeline-0"}"#);
    }
}
