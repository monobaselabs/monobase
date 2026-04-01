# Notification System Documentation

## Overview

The Monobase Rust API implements a multi-channel notification system that ensures patients, providers, and administrators stay informed throughout all healthcare workflows. The system supports in-app, push, and email delivery with non-blocking error handling.

## Architecture

### Core Components

- **NotificationService** (`src/service/notifs.rs`): Central service for push notification delivery via OneSignal
- **EmailService** (`src/service/email.rs`): Email delivery (delegated from NotificationService for email channel)
- **WebSocketService** (`src/service/ws.rs`): In-app real-time delivery via broadcast channels
- **Non-blocking design**: Notification failures don't interrupt primary workflows
- **Structured logging**: Full audit trail via `tracing`

### Integration Points

The notification system integrates across all major healthcare workflows:
- Appointment booking lifecycle
- Payment processing workflows
- Provider confirmation processes
- Automated background jobs

## NotificationService

Location: `src/service/notifs.rs`

```rust
pub struct NotificationService {
    onesignal_app_id: Option<String>,
    onesignal_api_key: Option<String>,
}

impl NotificationService {
    pub fn new(config: &Config) -> Self { /* ... */ }
    pub fn is_configured(&self) -> bool {
        self.onesignal_app_id.is_some() && self.onesignal_api_key.is_some()
    }
}
```

### Push Notifications via OneSignal

The service calls the OneSignal REST API via `reqwest`. Targeting uses OneSignal's `external_id` field (mapped to the Monobase person ID):

```rust
pub async fn send_push(
    &self,
    external_id: &str,   // Monobase person ID
    title: &str,
    message: &str,
    data: Option<serde_json::Value>,
) -> Result<String, String>
```

The `reqwest::Client` call posts to `https://onesignal.com/api/v1/notifications` with `Authorization: Basic {ONESIGNAL_API_KEY}`.

### Multi-channel Delivery

```rust
pub async fn deliver(
    &self,
    channel: &str,
    recipient_id: &str,
    title: &str,
    message: &str,
    data: Option<serde_json::Value>,
) -> Result<(), String> {
    match channel {
        "push"   => { self.send_push(recipient_id, title, message, data).await?; }
        "email"  => { /* delegate to EmailService */ }
        "in-app" => { /* stored in DB, delivered via WebSocketService */ }
        _        => return Err(format!("Unknown notification channel: {}", channel)),
    }
    Ok(())
}
```

## Notification Types

### 1. Appointment Workflow Notifications

#### 1.1 Appointment Confirmed (`appointment_confirmed`)
**Trigger**: Provider confirms a pending appointment
**Recipients**: Patient
**Channels**: in-app, email, SMS
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "providerId": "string",
  "confirmedAt": "ISO timestamp",
  "scheduledAt": "ISO timestamp"
}
```

**Message**: "Your appointment has been confirmed by the provider."

#### 1.2 Appointment Confirmation Sent (`appointment_confirmation_sent`)
**Trigger**: Provider confirms a pending appointment
**Recipients**: Provider
**Channels**: in-app
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "clientId": "string",
  "confirmedAt": "ISO timestamp"
}
```

**Message**: "Appointment confirmation has been sent to the patient."

#### 1.3 Appointment Cancelled by You (`appointment_cancelled_by_you`)
**Trigger**: User cancels their own appointment
**Recipients**: Cancelling user (patient or provider)
**Channels**: in-app
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "reason": "string",
  "cancelledAt": "ISO timestamp",
  "scheduledAt": "ISO timestamp",
  "cancelledBy": "client | provider"
}
```

**Message**: "You have successfully cancelled your appointment. The [other party] has been notified."

#### 1.4 Appointment Cancelled by Other (`appointment_cancelled_by_other`)
**Trigger**: Other party cancels the appointment
**Recipients**: Non-cancelling party
**Channels**: in-app, email, SMS
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "reason": "string",
  "cancelledAt": "ISO timestamp",
  "scheduledAt": "ISO timestamp",
  "cancelledBy": "client | provider",
  "cancelledById": "string"
}
```

**Message**: "Your appointment has been cancelled by the [canceller]. Reason: [reason]"

#### 1.5 Appointment Rejected (`appointment_rejected`)
**Trigger**: Provider rejects a pending appointment
**Recipients**: Patient
**Channels**: in-app, email, SMS
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "providerId": "string",
  "rejectedAt": "ISO timestamp",
  "scheduledAt": "ISO timestamp",
  "reason": "string",
  "slotReleased": "slot ID"
}
```

**Message**: "Your appointment request has been rejected by the provider. Reason: [reason]"

#### 1.6 Appointment Rejection Sent (`appointment_rejection_sent`)
**Trigger**: Provider rejects a pending appointment
**Recipients**: Provider
**Channels**: in-app
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "clientId": "string",
  "rejectedAt": "ISO timestamp",
  "reason": "string",
  "slotReleased": "slot ID"
}
```

**Message**: "Appointment rejection has been sent to the patient. The time slot is now available."

#### 1.7 Appointment Auto-Rejected (`appointment_auto_rejected`)
**Trigger**: Confirmation timer job — provider doesn't confirm within 15 minutes
**Recipients**: Patient
**Channels**: in-app, email, SMS
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "providerId": "string",
  "scheduledAt": "ISO timestamp",
  "autoRejectedAt": "ISO timestamp",
  "reason": "Provider did not confirm within 15 minutes"
}
```

**Message**: "Your appointment request has expired as the provider did not confirm within 15 minutes."

#### 1.8 Appointment Expired (`appointment_expired`)
**Trigger**: Confirmation timer job — provider doesn't confirm within 15 minutes
**Recipients**: Provider
**Channels**: in-app, email
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "clientId": "string",
  "scheduledAt": "ISO timestamp",
  "autoRejectedAt": "ISO timestamp",
  "missedDeadline": true
}
```

**Message**: "An appointment request has expired due to no confirmation within the time limit."

### 2. Payment Workflow Notifications

#### 2.1 Payment Authorized (`payment_authorized`)
**Trigger**: Stripe webhook — `payment_intent.succeeded` event
**Recipients**: Patient
**Channels**: in-app, email
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "paymentIntentId": "string",
  "amount": 0,
  "currency": "usd",
  "status": "authorized"
}
```

**Message**: "Your payment has been authorized and is being held until the appointment is completed."

#### 2.2 Payment Captured (`payment_captured`)
**Trigger**: Stripe webhook — `charge.succeeded` event
**Recipients**: Patient
**Channels**: in-app, email
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "chargeId": "string",
  "amount": 0,
  "currency": "usd",
  "status": "captured",
  "capturedAt": "ISO timestamp"
}
```

**Message**: "Your payment has been successfully processed and captured."

#### 2.3 Payment Received (`payment_received`)
**Trigger**: Stripe webhook — `charge.succeeded` event
**Recipients**: Provider
**Channels**: in-app, email
**Priority**: normal

**Data Payload**:
```json
{
  "appointmentId": "string",
  "chargeId": "string",
  "transferId": "string",
  "amount": 0,
  "currency": "usd",
  "clientId": "string"
}
```

**Message**: "Payment for your appointment has been processed and will be transferred to your account."

#### 2.4 Payment Failed (`payment_failed`)
**Trigger**: Stripe webhook — `payment_intent.payment_failed` event
**Recipients**: Patient
**Channels**: in-app, email, SMS
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "paymentIntentId": "string",
  "failureReason": "string",
  "status": "failed",
  "failedAt": "ISO timestamp"
}
```

**Message**: "Your payment could not be processed. Please update your payment method and try again."

#### 2.5 Charge Failed (`charge_failed`)
**Trigger**: Stripe webhook — `charge.failed` event
**Recipients**: Patient
**Channels**: in-app, email
**Priority**: high

**Data Payload**:
```json
{
  "appointmentId": "string",
  "chargeId": "string",
  "paymentIntentId": "string",
  "failureCode": "string",
  "failureMessage": "string",
  "status": "failed",
  "failedAt": "ISO timestamp"
}
```

**Message**: "There was an issue processing your payment charge. Please contact support if this continues."

## Implementation Patterns

### 1. Accessing the Service

`NotificationService` is available on the `AppContext`:

```rust
pub async fn confirm_appointment(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(appointment_id): Path<Uuid>,
) -> Result<Json<AppointmentResponse>, ApiError> {
    // ... business logic ...

    // Non-blocking notification
    let notifs = ctx.notifs.clone();
    tokio::spawn(async move {
        if let Err(e) = notifs.deliver(
            "push",
            &patient_id,
            "Appointment Confirmed",
            "Your appointment has been confirmed by the provider.",
            Some(serde_json::json!({ "appointmentId": appointment_id })),
        ).await {
            tracing::error!(error = %e, "Failed to send push notification");
        }
    });

    Ok(Json(appointment.into()))
}
```

### 2. Non-Blocking Error Handling

All notification sends use `tokio::spawn` so failures don't interrupt the primary handler:

```rust
let notifs = ctx.notifs.clone();
tokio::spawn(async move {
    match notifs.deliver("push", &recipient_id, title, message, data).await {
        Ok(_) => tracing::info!(recipient = %recipient_id, "Notification sent"),
        Err(e) => tracing::error!(error = %e, "Notification send failed"),
    }
});
```

### 3. In-App Notifications via WebSocket

In-app notifications are stored in the database and then published through the `WebSocketService`:

```rust
// Store in DB, then publish via WebSocket
ctx.ws.publish_to_user(
    &recipient_id,
    "notification.new",
    serde_json::json!({
        "id": notification_id,
        "type": notification_type,
        "title": title,
        "message": message,
    }),
).await;
```

### 4. Audit Logging

Each notification includes structured `tracing` fields:

```rust
tracing::info!(
    provider = "onesignal",
    external_id = external_id,
    title = title,
    "Push notification sent"
);
```

## Configuration

### Environment Variables

```bash
ONESIGNAL_APP_ID=your-app-id
ONESIGNAL_API_KEY=your-rest-api-key
```

When these are absent, `is_configured()` returns `false` and push delivery is skipped with a logged warning.

### OneSignal App-Agnostic Pattern

OneSignal uses `external_id` (the Monobase person ID) to target users across devices and apps. A single `ONESIGNAL_APP_ID` is shared across all frontends:

- Frontend apps set `VITE_ONESIGNAL_APP_ID` to the same value
- The Rust API uses `ONESIGNAL_APP_ID` to send notifications
- Users with both patient and provider roles receive notifications in whichever app they're currently using

## Channel Priority

- **High Priority Notifications**: in-app + email + SMS
  - Payment failures
  - Appointment rejections
  - Auto-rejections
  - Cancellations (to affected party)

- **Normal Priority Notifications**: in-app + email
  - Payment confirmations
  - Provider acknowledgments

- **Internal Notifications**: in-app only
  - Provider confirmations
  - Administrative notifications

## Troubleshooting

### Common Issues

1. **OneSignal Not Configured**
   - Verify `ONESIGNAL_APP_ID` and `ONESIGNAL_API_KEY` are set in the environment
   - Check `NotificationService::is_configured()` returns `true` at startup

2. **Push Notifications Not Received**
   - Confirm the `external_id` matches the person ID registered in OneSignal
   - Check OneSignal dashboard for delivery logs
   - Verify the app has push permission granted by the user

3. **Missing In-App Notifications**
   - Check the WebSocket connection is active for the recipient
   - Verify `WebSocketService::publish_to_user` returns `true` (user connected)

### Debugging

```bash
# Enable debug logging to see all notification attempts
RUST_LOG=debug cargo run

# Example output:
# DEBUG Creating notification provider=onesignal external_id="user-123"
# INFO  Push notification sent provider=onesignal external_id="user-123" title="Appointment Confirmed"
```

## Future Enhancements

1. **Template System**: Configurable notification templates stored in the database
2. **User Preferences**: Per-user channel preferences (opt-out of SMS, etc.)
3. **Delivery Confirmation**: Read receipts and OneSignal delivery webhooks
4. **Batch Notifications**: Efficient bulk notification sending
5. **Advanced Scheduling**: Timezone-aware notification scheduling
