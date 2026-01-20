use async_trait::async_trait;
use axum::{
    Router,
    extract::{Json, Query},
    routing::get,
};
use rmcp::model::ErrorData;
use std::sync::Arc;

/// Trait for HTTP operations that can be automatically registered
///
/// This trait enables automatic HTTP endpoint registration by providing
/// a uniform interface for all operations regardless of their specific
/// request/response types.
#[async_trait]
pub trait HttpOperation: Send + Sync + 'static {
    /// The HTTP path for this operation (e.g., "/api/tasks")
    fn path(&self) -> &'static str;

    /// Human-readable description of this operation
    fn description(&self) -> &'static str;

    /// Execute the operation with a JSON value
    ///
    /// This method performs type erasure by accepting and returning JSON values,
    /// allowing dynamic dispatch across different operation types.
    async fn execute_json(&self, json: serde_json::Value) -> Result<serde_json::Value, ErrorData>;
}

/// Register an HTTP operation on a router
///
/// Creates both GET and POST routes for the operation at its specified path.
/// The router state type must remain generic to work with the application's state.
pub fn register_operation<S>(router: Router<S>, operation: Arc<dyn HttpOperation>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let path = operation.path();
    let op_get = operation.clone();
    let op_post = operation;

    router.route(
        path,
        get({
            move |Query(params): Query<serde_json::Map<String, serde_json::Value>>| {
                let op = op_get.clone();
                async move {
                    let json_request = serde_json::Value::Object(params);
                    let json_response = op.execute_json(json_request).await.map_err(|e| {
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Operation failed: {}", e.message),
                        )
                    })?;
                    Ok::<_, (axum::http::StatusCode, String)>(Json(json_response))
                }
            }
        })
        .post({
            move |Json(json_request): Json<serde_json::Value>| {
                let op = op_post.clone();
                async move {
                    let json_response = op.execute_json(json_request).await.map_err(|e| {
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Operation failed: {}", e.message),
                        )
                    })?;
                    Ok::<_, (axum::http::StatusCode, String)>(Json(json_response))
                }
            }
        }),
    )
}
