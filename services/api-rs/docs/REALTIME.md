# Real-time Communication (WebSocket)

## Overview

The platform uses WebSocket connections for real-time features like notifications, chat, and video signaling. The Rust implementation uses **Axum's built-in WebSocket support** (`axum::extract::ws`) together with a `WebSocketService` that manages in-memory broadcast channels.

**Message Format**: All messages use a standard envelope:
```json
{ "event": "string", "payload": {} }
```

**Authentication**: WebSocket connections require Bearer token authentication in headers.

**Connection Types**:
- **User tracking**: 1:1 mapping (one broadcast channel per user)
- **Channel tracking**: Pub/sub (multiple connections per channel)

## WebSocketService

Location: `src/service/ws.rs`

```rust
pub struct WebSocketService {
    /// user_id → broadcast sender for direct messages
    user_channels: Arc<Mutex<HashMap<String, broadcast::Sender<WsMessage>>>>,
    /// channel_id → broadcast sender for channel messages
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<WsMessage>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub event: String,
    pub payload: serde_json::Value,
}
```

### API

```rust
// Register a user — returns a Receiver to pull messages from
pub async fn register_user(&self, user_id: &str) -> broadcast::Receiver<WsMessage>

// Unregister when the WebSocket closes
pub async fn unregister_user(&self, user_id: &str)

// Send a message to a specific user (returns false if not connected)
pub async fn publish_to_user(&self, user_id: &str, event: &str, payload: serde_json::Value) -> bool

// Subscribe to a named channel
pub async fn subscribe_channel(&self, channel_id: &str) -> broadcast::Receiver<WsMessage>

// Broadcast to all channel subscribers
pub async fn publish_to_channel(&self, channel_id: &str, event: &str, payload: serde_json::Value)

// Active connection stats: (users, channels)
pub async fn stats(&self) -> (usize, usize)
```

Broadcast channels are created with capacity 100. Slow receivers that fall behind will receive a `RecvError::Lagged` and should reconnect.

## WebSocket Routes

### `/ws/user` — Personal Notifications

Global connection for user-specific events.

**Authentication**: Required

**Client Sends**:
- `ping` — Heartbeat/keepalive

**Client Receives**:
- `connected` — Connection confirmation with `{ userId, timestamp }`
- `pong` — Heartbeat response with `{ timestamp }`
- `notification.new` — New notification created with `{ id, type, title, message, relatedEntityType, relatedEntity, createdAt }`
- `appointment.confirmed` — Appointment confirmed with `{ appointmentId, providerId, confirmedAt }`
- `appointment.cancelled` — Appointment cancelled with `{ appointmentId, cancelledBy, reason, cancelledAt }`
- `appointment.rejected` — Appointment rejected with `{ appointmentId, providerId, reason, rejectedAt }`

---

### `/ws/comms/chat-rooms/:room` — Chat & Video Signaling

Room-specific connection for chat messages and WebRTC signaling.

**Authentication**: Required + participant validation

**Client Sends**:
- `ping` — Heartbeat/keepalive
- `chat.message` — Send chat message with `{ text }`
- `chat.typing` — Typing indicator with `{ isTyping }`
- `video.offer` — WebRTC offer with `RTCSessionDescriptionInit`
- `video.answer` — WebRTC answer with `RTCSessionDescriptionInit`
- `video.ice-candidate` — ICE candidate with `RTCIceCandidateInit`

**Client Receives**:
- `connected` — Connection confirmation with `{ roomId, userId, timestamp }`
- `pong` — Heartbeat response with `{ timestamp }`
- `user.joined` — User joined room with `{ userId, timestamp }`
- `user.left` — User left room with `{ userId, timestamp }`
- `chat.message` — Chat message from peer (`id`, `chatRoom`, `sender`, `timestamp`, `messageType`, `message`, `videoCallData`, audit fields)
- `chat.typing` — Typing indicator from peer with `{ from, isTyping }`
- `video.offer` — WebRTC offer from peer with `{ type, from, data }`
- `video.answer` — WebRTC answer from peer with `{ type, from, data }`
- `video.ice-candidate` — ICE candidate from peer with `{ type, from, data }`

## Implementation Details

### Handler Pattern (Axum)

WebSocket handlers use `axum::extract::ws::WebSocketUpgrade`:

```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};
use axum::extract::{Path, State};
use axum::response::IntoResponse;

pub async fn ws_user_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_user_socket(socket, ctx, user))
}

async fn handle_user_socket(socket: WebSocket, ctx: AppContext, user: AuthUser) {
    let mut rx = ctx.ws.register_user(&user.id).await;
    let (mut sender, mut receiver) = socket.split();

    // Spawn task to forward broadcast messages to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            // Parse and route the message
        }
    }

    send_task.abort();
    ctx.ws.unregister_user(&user.id).await;
}
```

### Channel Namespacing

Channels are namespaced to avoid ID conflicts:
- `chat-rooms/{roomId}` for chat room channels
- `consultations/{consultationId}` for consultation channels

### Echo Prevention

To exclude the sender from channel broadcasts, each connected socket subscribes to the channel *receiver* independently. The handler compares the sending user's ID against the `from` field before forwarding:

```rust
// Relay to channel, excluding the sender
ctx.ws.publish_to_channel(
    &format!("chat-rooms/{room_id}"),
    "chat.message",
    serde_json::json!({ "from": user.id, "text": text }),
).await;
```

### Error Handling

Lifecycle errors are logged via `tracing`:

```rust
// Connection errors
tracing::error!(user_id = %user.id, error = %e, "WebSocket send error");

// Parse errors
tracing::warn!(user_id = %user.id, "Failed to parse WebSocket message");
```

## Client Integration

```javascript
// Connect with authentication
const ws = new WebSocket(`wss://api.example.com/ws/comms/chat-rooms/${roomId}`)
ws.onopen = () => {
  // Axum validates the token via middleware before upgrade
}

// Parse incoming messages
ws.onmessage = (event) => {
  const envelope = JSON.parse(event.data)

  if (envelope.event === 'connected') {
    console.log('Connected:', envelope.payload)
    return
  }

  if (envelope.event === 'user.joined' || envelope.event === 'user.left') {
    console.log('System event:', envelope.event)
    return
  }

  if (envelope.event === 'chat.message') {
    const { from, text, timestamp } = envelope.payload
    displayMessage(from, text)
  }

  if (envelope.event?.startsWith('video.')) {
    const { type, from, data } = envelope.payload
    handleSignaling(type, from, data)
  }
}

// Send messages
ws.send(JSON.stringify({ type: 'chat.message', data: { text: 'Hello!' } }))
ws.send(JSON.stringify({ type: 'video.offer', data: offerSDP }))
```

## Adding New WebSocket Handlers

1. Create handler file: `src/handlers/{module}/mod.rs` (or a dedicated `ws.rs` within the module)
2. Implement an `axum::extract::ws::WebSocketUpgrade` handler function
3. Register the route in `src/handlers/mod.rs`:

```rust
// src/handlers/mod.rs
pub fn router() -> Router<AppContext> {
    Router::new()
        // ... existing routes ...
        .route("/ws/example/:id", get(example_module::ws_handler))
}
```

**Handler skeleton**:

```rust
use axum::{
    extract::{Path, State, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, ctx, user, id))
}

async fn handle_socket(mut socket: WebSocket, ctx: AppContext, user: AuthUser, id: Uuid) {
    // Send connected confirmation
    let connected = serde_json::json!({
        "event": "connected",
        "payload": { "userId": user.id, "timestamp": chrono::Utc::now() }
    });
    let _ = socket.send(Message::Text(connected.to_string().into())).await;

    // Handle messages
    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(text) => { /* route message */ }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```
