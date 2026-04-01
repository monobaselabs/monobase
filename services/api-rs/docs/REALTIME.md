# Real-time Communication (WebSocket)

## Overview

The platform uses WebSocket connections for real-time features like notifications, chat, and video signaling.

**Message Format**: All messages use a standard envelope:
```typescript
{ event: string, payload?: any }
```

**Authentication**: WebSocket connections require Bearer token authentication in headers.

**Connection Types**:
- **User tracking**: 1:1 mapping (one connection per user)
- **Channel tracking**: Pub/sub (multiple connections per channel)

## WebSocket Routes

### `/ws/user` - Personal Notifications

Global connection for user-specific events.

**Authentication**: Required

**Client Sends**:
- `ping` - Heartbeat/keepalive

**Client Receives**:
- `connected` - Connection confirmation with `{ userId, timestamp }`
- `pong` - Heartbeat response with `{ timestamp }`
- `notification.new` - New notification created with `{ id, type, title, message, relatedEntityType, relatedEntity, createdAt }`
- `appointment.confirmed` - Appointment confirmed with `{ appointmentId, providerId, confirmedAt }`
- `appointment.cancelled` - Appointment cancelled with `{ appointmentId, cancelledBy, reason, cancelledAt }`
- `appointment.rejected` - Appointment rejected with `{ appointmentId, providerId, reason, rejectedAt }`

**Example**:
```typescript
ws.send(JSON.stringify({ type: 'ping' }))
// Receives: { event: 'pong', payload: { timestamp: '2024-...' } }
```

---

### `/ws/comms/chat-rooms/:room` - Chat & Video Signaling

Room-specific connection for chat messages and WebRTC signaling.

**Authentication**: Required + participant validation

**Client Sends**:
- `ping` - Heartbeat/keepalive
- `chat.message` - Send chat message with `{ text }`
- `chat.typing` - Typing indicator with `{ isTyping }`
- `video.offer` - WebRTC offer with `RTCSessionDescriptionInit`
- `video.answer` - WebRTC answer with `RTCSessionDescriptionInit`
- `video.ice-candidate` - ICE candidate with `RTCIceCandidateInit`

**Client Receives**:
- `connected` - Connection confirmation with `{ roomId, userId, timestamp }`
- `pong` - Heartbeat response with `{ timestamp }`
- `user.joined` - User joined room with `{ userId, timestamp }`
- `user.left` - User left room with `{ userId, timestamp }`
- `chat.message` - Chat message from peer (complete `ChatMessage` schema: `id`, `chatRoom`, `sender`, `timestamp`, `messageType`, `message`, `videoCallData`, audit fields)
- `chat.typing` - Typing indicator from peer with `{ from, isTyping }`
- `video.offer` - WebRTC offer from peer with `{ type, from, data }`
- `video.answer` - WebRTC answer from peer with `{ type, from, data }`
- `video.ice-candidate` - ICE candidate from peer with `{ type, from, data }`

**Example**:
```typescript
// Send chat message
ws.send(JSON.stringify({
  type: 'chat.message',
  data: { text: 'Hello!' }
}))

// Receive chat message (complete ChatMessage schema)
// { event: 'chat.message', payload: { id: '...', chatRoom: '...', sender: '...', timestamp: '...', messageType: 'text', message: 'Hello!', ... } }

// Send WebRTC offer
ws.send(JSON.stringify({
  type: 'video.offer',
  data: sessionDescription
}))

// Receive WebRTC offer from peer
// { event: 'video.offer', payload: { type: 'video.offer', from: 'user-id', data: {...} } }
```

## Implementation Details

### Channel Namespacing
Channels are namespaced by resource type to avoid ID conflicts:
- `chat-rooms/{roomId}` for chat room channels
- `consultations/{consultationId}` for consultation channels

### Echo Prevention
The sender is automatically excluded from channel broadcasts using unique WebSocket IDs stored on `ws.raw.__wsId`.

### Connection Tracking
- **Automatic**: Unique `__wsId` assigned on connection
- **User tracking**: `wsService.trackUser(userId, ws)`
- **Channel tracking**: `wsService.trackChannel(channelId, ws)`

### Error Handling
All lifecycle errors (onConnect, onMessage, onClose, onError) are centralized in the WebSocket registry with structured logging.

## Client Integration

```typescript
// Connect with authentication
const ws = new WebSocket(`wss://api.example.com/ws/comms/chat-rooms/${roomId}`, {
  headers: {
    Authorization: `Bearer ${token}`
  }
})

// Parse incoming messages
ws.onmessage = (event) => {
  const envelope = JSON.parse(event.data)

  // Filter system events
  if (envelope.event === 'connected') {
    console.log('Connected:', envelope.payload)
    return
  }

  if (envelope.event === 'user.joined' || envelope.event === 'user.left') {
    console.log('System event:', envelope.event)
    return
  }

  // Handle application events
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

## Adding New Handlers

1. Create handler file: `src/handlers/{module}/ws.{name}.ts`
2. Export `config: WebSocketHandler` with path, middleware, lifecycle methods
3. Run `bun run generate` to register the handler
4. Handlers are auto-registered in `src/generated/websocket/registry.ts`

**Handler template**:
```typescript
import type { Context } from 'hono';
import type { WSContext } from 'hono/ws';
import type { WebSocketHandler } from '@/core/ws';
import { authMiddleware } from '@/middleware/auth';

export const config: WebSocketHandler = {
  path: '/ws/example/:id',
  middleware: [authMiddleware()],

  async onConnect(ctx: Context, ws: WSContext) {
    // Track connection, send confirmation
  },

  async onMessage(ctx: Context, ws: WSContext, message: any) {
    // Handle incoming messages
  },

  async onClose(ctx: Context, ws: WSContext) {
    // Cleanup on disconnect
  },
};
```
