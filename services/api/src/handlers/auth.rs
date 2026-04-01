use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};

use crate::auth::{backend, session};
use crate::error::ApiError;
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub user: UserResponse,
}

/// POST /auth/sign-up/email
pub async fn sign_up_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SignUpRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()));
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok());

    let (user, token) = backend::sign_up_email(
        &state.db,
        &body.name,
        &body.email,
        &body.password,
        state.config.auth_session_expires_secs,
        ip,
        ua,
    )
    .await?;

    Ok(Json(AuthResponse {
        user: UserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            email_verified: user.email_verified,
            image: user.image,
        },
        token,
    }))
}

/// POST /auth/sign-in/email
pub async fn sign_in_email(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SignInRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()));
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok());

    let (user, token) = backend::sign_in_email(
        &state.db,
        &body.email,
        &body.password,
        state.config.auth_session_expires_secs,
        ip,
        ua,
    )
    .await?;

    Ok(Json(AuthResponse {
        user: UserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            email_verified: user.email_verified,
            image: user.image,
        },
        token,
    }))
}

/// POST /auth/sign-out
pub async fn sign_out(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract token and delete session
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let raw_token =
                    session::extract_bearer_token(auth_str, &state.config.auth_secret)?;
                session::delete_session(&state.db, &raw_token).await?;
            }
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /auth/get-session
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    Ok(Json(SessionResponse {
        user: UserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
            email_verified: user.email_verified,
            image: user.image,
        },
    }))
}
