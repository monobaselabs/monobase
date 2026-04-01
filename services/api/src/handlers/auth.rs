use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use crate::auth::backend::{
    authenticate, create_user, get_session, sign_out,
    Credentials, SignUpRequest,
};
use super::AppState;

/// POST /auth/sign-up/email
pub async fn sign_up_email(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SignUpRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match create_user(&state.db, &body).await {
        Ok(response) => Ok(Json(serde_json::to_value(response).unwrap())),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("unique") || msg.contains("duplicate") {
                Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "message": "User already exists" })),
                ))
            } else {
                tracing::error!("Sign-up error: {}", msg);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "message": "Sign-up failed" })),
                ))
            }
        }
    }
}

/// POST /auth/sign-in/email
pub async fn sign_in_email(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match authenticate(&state.db, &body).await {
        Ok(Some(response)) => Ok(Json(serde_json::to_value(response).unwrap())),
        Ok(None) => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "message": "Invalid email or password" })),
        )),
        Err(e) => {
            tracing::error!("Sign-in error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "message": "Sign-in failed" })),
            ))
        }
    }
}

/// POST /auth/sign-out
pub async fn sign_out_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    sign_out(&state.db, token).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /auth/session or POST /auth/get-session
pub async fn get_session_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match get_session(&state.db, token).await {
        Ok(Some(session)) => Ok(Json(session)),
        Ok(None) => Err(StatusCode::UNAUTHORIZED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
