# Billing Module Documentation

## Overview

The billing module implements invoice-based payments with Stripe Connect integration. It uses a **two-phase payment model**: authorization (hold) → capture (charge), giving providers control over payment timing.

**Key Pattern**: Payments are authorized upfront, held by Stripe, and captured only when the provider confirms service completion.

**For Stripe API details**, see [Stripe Connect docs](https://stripe.com/docs/connect) and [Payment Intents docs](https://stripe.com/docs/payments/payment-intents).

---

## Architecture

### Payment Flow

```
1. Invoice Creation → Create Stripe Payment Intent (authorize)
2. Payment Authorization → Funds held (not charged)
3. Provider Decision → Capture or cancel payment
4. Payment Capture → Funds transferred to provider
```

###  Merchant Account Onboarding

Providers must complete Stripe Connect onboarding before receiving payments:

```
1. Create Merchant Account → Register with Stripe Connect
2. Onboarding Link → Provider completes Stripe onboarding
3. Account Verification → Stripe enables charges & payouts
4. Ready for Payments → Can receive invoice payments
```

---

## Invoice State Machine

### Status Field
```
status: 'draft' | 'open' | 'paid' | 'void' | 'uncollectible'
```

- **draft**: Invoice created but not finalized
- **open**: Finalized and sent to customer (payment authorized)
- **paid**: Payment captured successfully
- **void**: Invoice canceled (payment released)
- **uncollectible**: Marked as uncollectible

### Payment Status Field
```
paymentStatus: 'unpaid' | 'processing' | 'requires_capture' | 'succeeded' | 'failed' | 'canceled'
```

- **unpaid**: No payment attempt
- **processing**: Payment being processed
- **requires_capture**: Authorized, awaiting provider decision
- **succeeded**: Payment captured
- **failed**: Payment authorization/capture failed
- **canceled**: Payment canceled

### State Transitions

```
# Valid transitions
draft → open (finalize invoice)
open → paid (capture payment)
open → void (cancel before capture)
open → uncollectible (mark as uncollectible)
```

---

## Stripe Integration

### BillingService

Location: `src/service/billing.rs`

The `BillingService` wraps the `async-stripe` crate (0.39). The inner `stripe::Client` is initialised lazily via `OnceLock` so service construction is infallible:

```rust
pub struct BillingService {
    secret_key: Option<String>,
    webhook_secret: Option<String>,
    stripe_url: Option<String>,   // Optional override for stripe-mock in tests
    client: OnceLock<Client>,
}

impl BillingService {
    pub fn new(config: &Config) -> Self { /* ... */ }
    pub fn is_configured(&self) -> bool { self.secret_key.is_some() }
}
```

### Webhook Signature Verification

```rust
pub fn verify_webhook(
    &self,
    payload: &[u8],
    signature: &str,
) -> Result<stripe::Event, BillingError> {
    let secret = self.webhook_secret.as_deref()
        .ok_or(BillingError::WebhookSecretMissing)?;

    let payload_str = std::str::from_utf8(payload)
        .map_err(|_| BillingError::WebhookInvalid(stripe::WebhookError::BadSignature))?;

    let event = Webhook::construct_event(payload_str, signature, secret)?;
    Ok(event)
}
```

### Payment Intents

```rust
pub async fn create_payment_intent(
    &self,
    amount: i64,
    currency: &str,
    metadata: HashMap<String, String>,
    transfer_to: Option<&str>,
) -> Result<PaymentIntent, BillingError> { /* ... */ }

pub async fn capture_payment_intent(
    &self,
    payment_intent_id: &str,
) -> Result<PaymentIntent, BillingError> { /* ... */ }

pub async fn cancel_payment_intent(
    &self,
    payment_intent_id: &str,
) -> Result<PaymentIntent, BillingError> { /* ... */ }
```

### Refunds

```rust
// Pass None for amount to refund the full remaining balance
pub async fn create_refund(
    &self,
    payment_intent_id: &str,
    amount: Option<i64>,
) -> Result<Refund, BillingError> { /* ... */ }
```

### Stripe Connect

```rust
pub async fn create_connect_account(&self, email: &str) -> Result<Account, BillingError> { /* ... */ }

pub async fn generate_onboarding_link(
    &self,
    account_id: &str,
    return_url: &str,
) -> Result<String, BillingError> { /* ... */ }
```

### Error Type

```rust
#[derive(Debug, Error)]
pub enum BillingError {
    #[error("Stripe is not configured (missing STRIPE_SECRET_KEY)")]
    NotConfigured,

    #[error("Webhook secret is not configured (missing STRIPE_WEBHOOK_SECRET)")]
    WebhookSecretMissing,

    #[error("Webhook signature verification failed: {0}")]
    WebhookInvalid(#[from] stripe::WebhookError),

    #[error("Stripe API error: {0}")]
    Stripe(#[from] stripe::StripeError),

    #[error("Invalid Stripe ID '{0}'")]
    InvalidId(String),
}
```

---

## Stripe Webhook Integration

### Webhook Handler

Location: `src/handlers/billing/mod.rs`

**Purpose**: Synchronize Stripe events with local database and trigger notifications.

### Handled Events

#### Payment Events
- **payment_intent.succeeded** → Mark invoice as `requires_capture`
- **payment_intent.payment_failed** → Mark as `failed`, notify customer
- **payment_intent.canceled** → Mark as `canceled` and void invoice
- **payment_intent.requires_action** → Mark as `processing` (3D Secure, etc.)

#### Charge Events
- **charge.succeeded** → Mark invoice as `paid`, notify both parties
- **charge.failed** → Mark as `failed`, notify customer
- **charge.refunded** → Update metadata with refund details

#### Connect Account Events
- **account.updated** → Update merchant account status and onboarding state
- **account.application.deauthorized** → Deactivate merchant account

#### Transfer Events
- **transfer.created** → Log successful transfer to merchant
- **transfer.failed** → Log failed transfer (requires manual review)

### Webhook Error Handling

The handler always returns 200 to prevent Stripe retries on business logic errors:

```rust
pub async fn handle_stripe_webhook(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("Missing Stripe-Signature header".into()))?;

    let event = ctx.billing.verify_webhook(&body, signature)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Dispatch on event.type_ — return 200 even for business logic errors
    match event.type_ {
        // ...
    }

    Ok(Json(serde_json::json!({ "received": true })))
}
```

---

## Two-Phase Payment Model

### Phase 1: Authorization (Hold)

**Endpoint**: `POST /billing/invoices` → finalize handler

```
// Customer pays → Stripe authorizes payment
// Funds are HELD, not charged
// Invoice status: open
// Payment status: requires_capture
```

**Benefits**:
- Provider can verify service completion before charging
- Customer sees authorization on statement
- No charge if service is canceled

### Phase 2: Capture (Charge)

**Endpoint**: `POST /billing/invoices/:id/capture`

```
// Provider confirms service → Capture payment
// Funds transferred to provider account
// Invoice status: paid
// Payment status: succeeded
```

**Alternative**: Cancel Instead
```
// Service canceled → Release authorization
// POST /billing/invoices/:id/void
// Invoice status: void
// Payment status: canceled
```

---

## Merchant Account Flow

### 1. Create Merchant Account

**Endpoint**: `POST /billing/merchant-accounts`

Creates Stripe Express connected account for provider.

### 2. Generate Onboarding Link

**Endpoint**: `POST /billing/merchant-accounts/:id/onboard`

Returns Stripe-hosted onboarding URL.

```json
{
  "onboardingUrl": "https://connect.stripe.com/setup/s/...",
  "expiresAt": "2024-01-15T10:00:00Z"
}
```

### 3. Monitor Onboarding Status

**Via Webhooks**: `account.updated` events update `metadata.onboardingComplete`

**Via API**: `GET /billing/merchant-accounts/:id`

```json
{
  "active": true,
  "metadata": {
    "onboardingComplete": true,
    "accountChargesEnabled": true,
    "accountPayoutsEnabled": true,
    "requirementsCurrentlyDue": []
  }
}
```

### 4. Access Merchant Dashboard

**Endpoint**: `POST /billing/merchant-accounts/:id/dashboard`

Returns Stripe Express Dashboard login link.

---

## Error Handling

### Business Logic Errors

```rust
// Invoice must be in correct state
if invoice.status != "open" {
    return Err(ApiError::BadRequest(
        "Cannot capture payment: invoice is not open".into()
    ));
}

// Merchant must be onboarded
if !merchant_account.active {
    return Err(ApiError::BadRequest(
        "Merchant account not active".into()
    ));
}
```

---

## Implementation Files

**Service**:
- `src/service/billing.rs` — `BillingService` wrapping `async-stripe`

**Handlers** (`src/handlers/billing/mod.rs`):
- `create_invoice` — Create draft invoice
- `finalize_invoice` — Finalize and authorize payment
- `capture_invoice_payment` — Capture authorized payment
- `pay_invoice` — Direct payment (skip authorization)
- `refund_invoice_payment` — Refund captured payment
- `void_invoice` — Cancel invoice and release authorization
- `handle_stripe_webhook` — Webhook event handler
- `onboard_merchant_account` — Generate onboarding link
- `get_merchant_dashboard` — Generate dashboard link

**Repository**:
- `src/handlers/billing/repo.rs` — SeaORM database operations

---

## Configuration

Environment variables (parsed into `Config` via clap):

```bash
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_URL=                    # Optional: override for stripe-mock in tests
```

---

## Testing

### Webhook Testing

Use Stripe CLI to forward webhooks:

```bash
stripe listen --forward-to localhost:7213/billing/stripe-webhook

# Trigger test events
stripe trigger payment_intent.succeeded
stripe trigger charge.succeeded
stripe trigger account.updated
```

### Running Tests

```bash
# Run all tests
cargo test

# Run billing-specific tests
cargo test billing

# Run with output
cargo test -- --nocapture
```

### Test Cards

```
4242 4242 4242 4242  # Successful payment
4000 0000 0000 9995  # Declined payment
4000 0025 0000 3155  # Requires authentication (3D Secure)
```

See [Stripe test cards](https://stripe.com/docs/testing) for complete list.

---

## Security Considerations

1. **Webhook Signature Verification**: Always verify Stripe signatures via `BillingService::verify_webhook`
2. **Idempotency**: Webhooks may be delivered multiple times
3. **Metadata Privacy**: Don't store PHI in Stripe metadata
4. **Audit Logging**: All payment operations logged with `tracing`
5. **Authorization Timing**: Authorizations expire after 7 days

---

## Common Patterns

### Creating Invoice with Authorization

```
1. Create draft invoice (POST /billing/invoices)
2. Finalize invoice — authorizes Stripe payment intent
   Result: Payment authorized, funds held
   Invoice status: open / Payment status: requires_capture
```

### Capturing Payment After Service

```
Provider confirms service completion
POST /billing/invoices/:id/capture
  → BillingService::capture_payment_intent(payment_intent_id)
Result: Payment captured, funds transferred
Invoice status: paid / Payment status: succeeded
```

### Canceling Before Capture

```
Service canceled before completion
POST /billing/invoices/:id/void
  → BillingService::cancel_payment_intent(payment_intent_id)
Result: Authorization released, no charge
Invoice status: void / Payment status: canceled
```

---

## Future Enhancements

- Partial payment capture
- Recurring billing/subscriptions
- Multi-currency support
- Platform fee configuration
- Automated dunning for failed payments

---

**For complete Stripe API reference**, see [Stripe Documentation](https://stripe.com/docs).
