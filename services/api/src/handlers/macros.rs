//! Macros for generating thin Axum handlers that delegate to CrudService.

/// Generate thin CRUD handlers for a service field on AppState.
///
/// Usage:
/// ```ignore
/// crud_handlers!(module_name, service_field);
/// ```
///
/// Generates: list, get, create, patch, delete handlers that delegate
/// to `state.{service_field}`.
macro_rules! crud_handlers {
    ($mod_name:ident, $service_field:ident) => {
        pub mod $mod_name {
            use axum::extract::{Path, RawQuery, State};
            use axum::http::{HeaderMap, StatusCode};
            use axum::Json;
            use serde_json::Value;
            use std::collections::HashMap;
            use std::sync::Arc;
            use crate::error::ApiError;
            use crate::handlers::AppState;
            use crate::service::ServiceContext;

            fn build_ctx(raw_query: &str) -> ServiceContext {
                let query = crate::handlers::generic::parse_qs_to_json(raw_query);
                ServiceContext {
                    user: None,
                    query,
                    params: HashMap::new(),
                    provider: "rest".to_string(),
                }
            }

            async fn extract_user(state: &AppState, headers: &HeaderMap) -> Option<crate::auth::backend::AuthUser> {
                crate::middleware::auth::extract_auth(state, headers).await
            }

            pub async fn list(
                State(state): State<Arc<AppState>>,
                headers: HeaderMap,
                raw_query: RawQuery,
            ) -> Result<Json<Value>, ApiError> {
                let mut ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""));
                ctx.user = extract_user(&state, &headers).await;
                let result = state.$service_field.find(&ctx).await?;
                Ok(Json(result))
            }

            pub async fn get(
                State(state): State<Arc<AppState>>,
                headers: HeaderMap,
                Path(id): Path<String>,
                raw_query: RawQuery,
            ) -> Result<Json<Value>, ApiError> {
                let mut ctx = build_ctx(raw_query.0.as_deref().unwrap_or(""));
                ctx.user = extract_user(&state, &headers).await;
                let result = state.$service_field.get(&id, &ctx).await?;
                Ok(Json(result))
            }

            pub async fn create(
                State(state): State<Arc<AppState>>,
                headers: HeaderMap,
                Json(body): Json<Value>,
            ) -> Result<(StatusCode, Json<Value>), ApiError> {
                let mut ctx = build_ctx("");
                ctx.user = extract_user(&state, &headers).await;
                let result = state.$service_field.create(body, &ctx).await?;
                Ok((StatusCode::CREATED, Json(result)))
            }

            pub async fn patch(
                State(state): State<Arc<AppState>>,
                headers: HeaderMap,
                Path(id): Path<String>,
                Json(body): Json<Value>,
            ) -> Result<Json<Value>, ApiError> {
                let mut ctx = build_ctx("");
                ctx.user = extract_user(&state, &headers).await;
                let result = state.$service_field.patch(&id, body, &ctx).await?;
                Ok(Json(result))
            }

            pub async fn delete(
                State(state): State<Arc<AppState>>,
                headers: HeaderMap,
                Path(id): Path<String>,
            ) -> Result<Json<Value>, ApiError> {
                let mut ctx = build_ctx("");
                ctx.user = extract_user(&state, &headers).await;
                let result = state.$service_field.remove(&id, &ctx).await?;
                Ok(Json(result))
            }
        }
    };
}

pub(crate) use crud_handlers;
