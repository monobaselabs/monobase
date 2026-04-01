use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
    routing::{any, delete, get, post, patch},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::handlers::AppState;
use crate::service::ws::WsMessage;

/// Create the main Axum router with all routes and middleware.
pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    let app = Router::new()
        // -- Health / internal --
        .route("/health", get(health))
        .route("/livez", get(health))
        .route("/readyz", get(health))
        // -- Auth --
        .route("/auth/sign-up/email", post(crate::handlers::auth::sign_up_email))
        .route("/auth/sign-in/email", post(crate::handlers::auth::sign_in_email))
        .route("/auth/sign-out", post(crate::handlers::auth::sign_out))
        .route("/auth/get-session", get(crate::handlers::auth::get_session))
        // -- Person --
        .route("/persons", post(crate::handlers::person::create_person))
        .route("/persons", get(crate::handlers::person::list_persons))
        .route("/persons/:person", get(crate::handlers::person::get_person))
        .route("/persons/:person", patch(crate::handlers::person::update_person))
        // -- Reviews --
        .route("/reviews", post(crate::handlers::reviews::create_review))
        .route("/reviews", get(crate::handlers::reviews::list_reviews))
        .route("/reviews/:review", get(crate::handlers::reviews::get_review))
        .route("/reviews/:review", delete(crate::handlers::reviews::delete_review))
        // -- Audit --
        .route("/audit/logs", get(crate::handlers::audit::list_audit_logs))
        // -- Billing / Invoices --
        .route("/billing/invoices", post(crate::handlers::billing::create_invoice))
        .route("/billing/invoices", get(crate::handlers::billing::list_invoices))
        .route("/billing/invoices/:invoice", get(crate::handlers::billing::get_invoice))
        .route("/billing/invoices/:invoice", patch(crate::handlers::billing::update_invoice))
        .route("/billing/invoices/:invoice", delete(crate::handlers::billing::delete_invoice))
        .route("/billing/invoices/:invoice/capture", post(crate::handlers::billing::capture_invoice_payment))
        .route("/billing/invoices/:invoice/finalize", post(crate::handlers::billing::finalize_invoice))
        .route("/billing/invoices/:invoice/mark-uncollectible", post(crate::handlers::billing::mark_invoice_uncollectible))
        .route("/billing/invoices/:invoice/pay", post(crate::handlers::billing::pay_invoice))
        .route("/billing/invoices/:invoice/refund", post(crate::handlers::billing::refund_invoice_payment))
        .route("/billing/invoices/:invoice/void", post(crate::handlers::billing::void_invoice))
        // -- Billing / Merchant Accounts --
        .route("/billing/merchant-accounts", post(crate::handlers::billing::create_merchant_account))
        .route("/billing/merchant-accounts/:merchantAccount", get(crate::handlers::billing::get_merchant_account))
        .route("/billing/merchant-accounts/:merchantAccount/dashboard", post(crate::handlers::billing::get_merchant_dashboard))
        .route("/billing/merchant-accounts/:merchantAccount/onboard", post(crate::handlers::billing::onboard_merchant_account))
        // -- Billing / Webhooks (public) --
        .route("/billing/webhooks/stripe", post(crate::handlers::billing::handle_stripe_webhook))
        // -- Booking / Bookings --
        .route("/booking/bookings", post(crate::handlers::booking::create_booking))
        .route("/booking/bookings", get(crate::handlers::booking::list_bookings))
        .route("/booking/bookings/:booking", get(crate::handlers::booking::get_booking))
        .route("/booking/bookings/:booking/cancel", post(crate::handlers::booking::cancel_booking))
        .route("/booking/bookings/:booking/confirm", post(crate::handlers::booking::confirm_booking))
        .route("/booking/bookings/:booking/no-show", post(crate::handlers::booking::mark_no_show))
        .route("/booking/bookings/:booking/reject", post(crate::handlers::booking::reject_booking))
        // -- Booking / Events --
        .route("/booking/events", post(crate::handlers::booking::create_booking_event))
        .route("/booking/events", get(crate::handlers::booking::list_booking_events))
        .route("/booking/events/:event", get(crate::handlers::booking::get_booking_event))
        .route("/booking/events/:event", patch(crate::handlers::booking::update_booking_event))
        .route("/booking/events/:event", delete(crate::handlers::booking::delete_booking_event))
        // -- Booking / Schedule Exceptions --
        .route("/booking/events/:event/exceptions", post(crate::handlers::booking::create_schedule_exception))
        .route("/booking/events/:event/exceptions", get(crate::handlers::booking::list_schedule_exceptions))
        .route("/booking/events/:event/exceptions/:exception", get(crate::handlers::booking::get_schedule_exception))
        .route("/booking/events/:event/exceptions/:exception", delete(crate::handlers::booking::delete_schedule_exception))
        // -- Booking / Slots --
        .route("/booking/events/:event/slots", get(crate::handlers::booking::list_event_slots))
        .route("/booking/slots/:slotId", get(crate::handlers::booking::get_time_slot))
        // -- Storage --
        .route("/storage/files", get(crate::handlers::storage::list_files))
        .route("/storage/files/upload", post(crate::handlers::storage::upload_file))
        .route("/storage/files/:file", get(crate::handlers::storage::get_file))
        .route("/storage/files/:file", delete(crate::handlers::storage::delete_file))
        .route("/storage/files/:file/complete", post(crate::handlers::storage::complete_file_upload))
        .route("/storage/files/:file/download", get(crate::handlers::storage::get_file_download))
        // -- Notifications --
        .route("/notifs", get(crate::handlers::notifs::list_notifications))
        .route("/notifs/read-all", post(crate::handlers::notifs::mark_all_notifications_as_read))
        .route("/notifs/:notif", get(crate::handlers::notifs::get_notification))
        .route("/notifs/:notif/read", post(crate::handlers::notifs::mark_notification_as_read))
        // -- Patient --
        .route("/patients", post(crate::handlers::patient::create_patient))
        .route("/patients", get(crate::handlers::patient::list_patients))
        .route("/patients/:patient", get(crate::handlers::patient::get_patient))
        .route("/patients/:patient", patch(crate::handlers::patient::update_patient))
        .route("/patients/:patient", delete(crate::handlers::patient::delete_patient))
        // -- Provider --
        .route("/providers", post(crate::handlers::provider::create_provider))
        .route("/providers", get(crate::handlers::provider::list_providers))
        .route("/providers/:provider", get(crate::handlers::provider::get_provider))
        .route("/providers/:provider", patch(crate::handlers::provider::update_provider))
        .route("/providers/:provider", delete(crate::handlers::provider::delete_provider))
        // -- Communications --
        .route("/comms/chat-rooms", post(crate::handlers::comms::create_chat_room))
        .route("/comms/chat-rooms", get(crate::handlers::comms::list_chat_rooms))
        .route("/comms/chat-rooms/:room", get(crate::handlers::comms::get_chat_room))
        .route("/comms/chat-rooms/:room/messages", get(crate::handlers::comms::get_chat_messages))
        .route("/comms/chat-rooms/:room/messages", post(crate::handlers::comms::send_chat_message))
        .route("/comms/chat-rooms/:room/video-call/join", post(crate::handlers::comms::join_video_call))
        .route("/comms/chat-rooms/:room/video-call/leave", post(crate::handlers::comms::leave_video_call))
        .route("/comms/chat-rooms/:room/video-call/end", post(crate::handlers::comms::end_video_call))
        .route("/comms/chat-rooms/:room/video-call/participant", patch(crate::handlers::comms::update_video_call_participant))
        .route("/comms/ice-servers", get(crate::handlers::comms::get_ice_servers))
        // -- EMR --
        .route("/emr/consultations", post(crate::handlers::emr::create_consultation))
        .route("/emr/consultations", get(crate::handlers::emr::list_consultations))
        .route("/emr/consultations/:consultation", get(crate::handlers::emr::get_consultation))
        .route("/emr/consultations/:consultation", patch(crate::handlers::emr::update_consultation))
        .route("/emr/consultations/:consultation/finalize", post(crate::handlers::emr::finalize_consultation))
        .route("/emr/patients", get(crate::handlers::emr::list_emr_patients))
        // -- Email --
        .route("/email/queue", get(crate::handlers::email::list_email_queue_items))
        .route("/email/queue/:queue", get(crate::handlers::email::get_email_queue_item))
        .route("/email/queue/:queue/cancel", post(crate::handlers::email::cancel_email_queue_item))
        .route("/email/queue/:queue/retry", post(crate::handlers::email::retry_email_queue_item))
        .route("/email/templates", get(crate::handlers::email::list_email_templates))
        .route("/email/templates", post(crate::handlers::email::create_email_template))
        .route("/email/templates/:template", get(crate::handlers::email::get_email_template))
        .route("/email/templates/:template", patch(crate::handlers::email::update_email_template))
        .route("/email/templates/:template/test", post(crate::handlers::email::test_email_template))
        // -- Documentation --
        .route("/docs/openapi.json", get(serve_openapi_spec))
        .route("/docs", get(serve_docs_ui))
        // -- WebSocket --
        .route("/ws/user", any(ws_user_handler))
        .route("/ws/comms/chat-rooms/:room", any(ws_chat_room_handler))
        //
        .with_state(state);

    // Apply middleware layers (outermost first)
    app.layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "pass" }))
}

// ---------------------------------------------------------------------------
// Documentation
// ---------------------------------------------------------------------------

/// GET /docs/openapi.json — serve the OpenAPI spec.
///
/// Tries to load from specs/api/dist/openapi/openapi.json at runtime.
/// Falls back to a minimal spec describing the health endpoints.
async fn serve_openapi_spec(
    State(state): State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    // Try to load the TypeSpec-generated spec at runtime
    static SPEC: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();

    let spec = SPEC.get_or_init(|| {
        // Look for the spec relative to CWD (typical in dev and Docker)
        let paths = [
            "../../specs/api/dist/openapi/openapi.json",
            "../specs/api/dist/openapi/openapi.json",
            "specs/api/dist/openapi/openapi.json",
        ];
        for path in &paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(spec) = serde_json::from_str::<serde_json::Value>(&content) {
                    tracing::info!(path = %path, "Loaded OpenAPI spec from file");
                    return spec;
                }
            }
        }

        tracing::warn!("OpenAPI spec not found on disk — serving minimal spec");
        let base_url = state.config.public_url();
        serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Monobase API",
                "version": "0.1.0",
                "description": "Monobase Application Platform API (Rust)"
            },
            "servers": [{ "url": base_url, "description": "Current server" }],
            "paths": {
                "/health": {
                    "get": {
                        "summary": "Health check",
                        "operationId": "healthCheck",
                        "responses": { "200": { "description": "OK" } }
                    }
                }
            }
        })
    });

    axum::Json(spec.clone())
}

/// GET /docs — serve the Scalar interactive API documentation UI.
async fn serve_docs_ui() -> axum::response::Html<String> {
    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Monobase API Documentation</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content="Monobase Application Platform API Documentation" />
</head>
<body>
    <script id="api-reference" data-url="/docs/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#
    ))
}

// ---------------------------------------------------------------------------
// WebSocket — query params
// ---------------------------------------------------------------------------

/// Query parameters accepted by both WS upgrade endpoints.
///
/// The `token` carries the session JWT because WebSocket upgrade requests
/// cannot attach arbitrary headers in the browser.
#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

// ---------------------------------------------------------------------------
// GET /ws/user
//
// Personal notification WebSocket for an authenticated user.
// ---------------------------------------------------------------------------

async fn ws_user_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<WsQuery>,
) -> Response {
    // Resolve the user ID from the token (best-effort; unauthenticated
    // connections still get a connection but receive no targeted messages).
    let user_id = resolve_user_id_from_token(&params.token, &state.config.auth_secret);

    ws.on_upgrade(move |socket| handle_user_ws(socket, state, user_id))
}

async fn handle_user_ws(socket: WebSocket, state: Arc<AppState>, user_id: Option<String>) {
    let (mut sender, mut receiver) = socket.split();

    // If no valid token was provided, send an auth error and close.
    let user_id = match user_id {
        Some(id) => id,
        None => {
            let _ = sender
                .send(Message::Text(
                    serde_json::json!({
                        "event": "error",
                        "payload": { "code": "UNAUTHENTICATED", "message": "A valid token query parameter is required" }
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    tracing::debug!(user_id = %user_id, "User WebSocket connected");

    // Register with the WS service and obtain a broadcast receiver.
    let mut rx = state.ws.register_user(&user_id).await;

    // Forward incoming service messages to the WebSocket client.
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let text = serde_json::json!({
                        "event": msg.event,
                        "payload": msg.payload,
                    })
                    .to_string();
                    if sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                // The sender was dropped (no more senders) or lagged; disconnect.
                Err(_) => break,
            }
        }
    });

    // Drain incoming client frames (ping/pong, close, echo — ignored for now).
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                _ => {} // client→server messages not used on this channel
            }
        }
    });

    // When either direction closes, abort the other.
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    state.ws.unregister_user(&user_id).await;
    tracing::debug!(user_id = %user_id, "User WebSocket disconnected");
}

// ---------------------------------------------------------------------------
// GET /ws/comms/chat-rooms/:room
//
// Chat room broadcast WebSocket.
// ---------------------------------------------------------------------------

async fn ws_chat_room_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Query(params): Query<WsQuery>,
) -> Response {
    let user_id = resolve_user_id_from_token(&params.token, &state.config.auth_secret);

    ws.on_upgrade(move |socket| handle_chat_room_ws(socket, state, room_id, user_id))
}

async fn handle_chat_room_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    room_id: String,
    user_id: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    let user_id = match user_id {
        Some(id) => id,
        None => {
            let _ = sender
                .send(Message::Text(
                    serde_json::json!({
                        "event": "error",
                        "payload": { "code": "UNAUTHENTICATED", "message": "A valid token query parameter is required" }
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            return;
        }
    };

    tracing::debug!(user_id = %user_id, room_id = %room_id, "Chat room WebSocket connected");

    // Subscribe to channel broadcasts.
    let mut rx = state.ws.subscribe_channel(&room_id).await;

    // Forward channel messages → client.
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let text = serde_json::json!({
                        "event": msg.event,
                        "payload": msg.payload,
                    })
                    .to_string();
                    if sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Forward client messages → channel.
    let ws_svc = Arc::clone(&state.ws);
    let room_id_clone = room_id.clone();
    let uid_clone = user_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse the incoming envelope: { "event": "...", "payload": {...} }
                    if let Ok(parsed) = serde_json::from_str::<WsMessage>(&text) {
                        ws_svc
                            .publish_to_channel(
                                &room_id_clone,
                                &parsed.event,
                                serde_json::json!({
                                    "sender": uid_clone,
                                    "data": parsed.payload,
                                }),
                            )
                            .await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    tracing::debug!(user_id = %user_id, room_id = %room_id, "Chat room WebSocket disconnected");
}

// ---------------------------------------------------------------------------
// Auth helpers
// ---------------------------------------------------------------------------

/// Attempt to extract a user ID from a bearer JWT `token` query param.
///
/// Returns `None` when the token is absent, malformed, or the signature is
/// invalid — callers decide whether to reject or degrade gracefully.
fn resolve_user_id_from_token(token: &Option<String>, secret: &str) -> Option<String> {
    use jsonwebtoken::{DecodingKey, Validation, decode};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Claims {
        sub: String,
    }

    let tok = token.as_deref()?;
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::default();
    validation.validate_exp = true;

    decode::<Claims>(tok, &key, &validation)
        .ok()
        .map(|data| data.claims.sub)
}
