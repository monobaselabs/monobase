use axum::{
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{auth, generic, internal, AppState};

/// Create the Axum HTTP router.
///
/// Auth enforcement is handled per-handler (via extract_user + ServiceContext),
/// NOT via blanket middleware.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // === Internal / Health (public) ===
        .route("/livez", get(internal::livez))
        .route("/health", get(internal::livez))
        .route("/readyz", get(internal::readyz))
        .route("/.well-known/jwks.json", get(internal::jwks))
        .route("/internal/test-reset", post(internal::test_reset))

        // === Auth (public) ===
        .route("/auth/sign-up/email", post(auth::sign_up_email))
        .route("/auth/sign-in/email", post(auth::sign_in_email))
        .route("/auth/sign-out", post(auth::sign_out_handler))
        .route("/auth/get-session", post(auth::get_session_handler))
        .route("/auth/session", get(auth::get_session_handler))

        // === Generic CRUD catch-all ===
        // 3-segment: /prefix/subtable/id (e.g., /billing/invoices/abc123)
        .route("/{prefix}/{subtable}/{id}", get(generic::generic_nested_get).patch(generic::generic_nested_patch).delete(generic::generic_nested_delete))
        // 1-segment: /table (e.g., /organizations)
        .route("/{table}", get(generic::generic_list).post(generic::generic_create).delete(generic::generic_bulk_delete))
        // 2-segment: smart handler that detects nested list vs get-by-id
        .route("/{table}/{id_or_subtable}", get(generic::generic_smart_get).post(generic::generic_nested_create).patch(generic::generic_patch).delete(generic::generic_delete))

        // === Middleware ===
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
