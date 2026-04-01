//! Stripe billing service.
//!
//! Wraps the async-stripe crate (0.39) for payment intent management,
//! Stripe Connect onboarding, and webhook signature verification.
//!
//! The [`stripe::Client`] is constructed lazily on first use so that the
//! service can be created at startup without blocking on network I/O.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;

use stripe::{
    Account, AccountId, AccountLink, AccountLinkType, AccountType, Client, CreateAccount,
    CreateAccountLink, CreatePaymentIntent, CreatePaymentIntentTransferData, CreateRefund,
    Currency, PaymentIntent, PaymentIntentId, Refund, Webhook,
};
use thiserror::Error;

use crate::config::Config;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Wraps the Stripe client and provides high-level billing operations.
///
/// The inner [`stripe::Client`] is built on first call to [`client()`] so that
/// service construction is infallible and zero-cost when Stripe is not
/// configured.
pub struct BillingService {
    secret_key: Option<String>,
    webhook_secret: Option<String>,
    /// Optional base URL override (e.g. a stripe-mock instance for testing).
    stripe_url: Option<String>,
    /// Lazily initialised client — built at most once.
    client: OnceLock<Client>,
}

impl BillingService {
    pub fn new(config: &Config) -> Self {
        Self {
            secret_key: config.stripe_secret_key.clone(),
            webhook_secret: config.stripe_webhook_secret.clone(),
            stripe_url: config.stripe_url.clone(),
            client: OnceLock::new(),
        }
    }

    /// Returns `true` when a Stripe secret key has been provided.
    pub fn is_configured(&self) -> bool {
        self.secret_key.is_some()
    }

    /// Returns a reference to the lazily-initialised [`stripe::Client`].
    fn client(&self) -> Result<&Client, BillingError> {
        let key = self.secret_key.as_deref().ok_or(BillingError::NotConfigured)?;

        Ok(self.client.get_or_init(|| {
            match &self.stripe_url {
                Some(url) => Client::from_url(url.as_str(), key),
                None => Client::new(key),
            }
        }))
    }

    // -----------------------------------------------------------------------
    // Webhook
    // -----------------------------------------------------------------------

    /// Verify and parse a Stripe webhook event.
    ///
    /// `payload` is the raw request body bytes; `signature` is the value of
    /// the `Stripe-Signature` HTTP header.
    ///
    /// Returns the parsed [`stripe::Event`] on success so callers can dispatch
    /// on `event.type_` without re-parsing the body.
    pub fn verify_webhook(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> Result<stripe::Event, BillingError> {
        let secret = self
            .webhook_secret
            .as_deref()
            .ok_or(BillingError::WebhookSecretMissing)?;

        // Stripe expects the payload as a UTF-8 string.
        let payload_str = std::str::from_utf8(payload)
            .map_err(|_| BillingError::WebhookInvalid(stripe::WebhookError::BadSignature))?;

        let event = Webhook::construct_event(payload_str, signature, secret)?;
        Ok(event)
    }

    // -----------------------------------------------------------------------
    // Payment intents
    // -----------------------------------------------------------------------

    /// Create a new payment intent.
    ///
    /// * `amount`       – amount in the smallest currency unit (e.g. cents).
    /// * `currency`     – three-letter ISO currency code such as `"usd"`.
    /// * `metadata`     – arbitrary key/value pairs attached to the intent.
    /// * `transfer_to`  – optional connected-account ID for Stripe Connect
    ///                    destination charges.
    pub async fn create_payment_intent(
        &self,
        amount: i64,
        currency: &str,
        metadata: HashMap<String, String>,
        transfer_to: Option<&str>,
    ) -> Result<PaymentIntent, BillingError> {
        let client = self.client()?;

        // `Currency` implements `FromStr`; fall back to USD on unknown codes
        // so callers get a clear runtime error from Stripe rather than a panic.
        let currency = Currency::from_str(currency).unwrap_or(Currency::USD);

        let mut params = CreatePaymentIntent::new(amount, currency);
        params.metadata = Some(metadata);

        if let Some(destination) = transfer_to {
            params.transfer_data = Some(CreatePaymentIntentTransferData {
                destination: destination.to_owned(),
                ..Default::default()
            });
        }

        let intent = PaymentIntent::create(client, params).await?;
        Ok(intent)
    }

    /// Capture a payment intent that was created with `capture_method=manual`.
    pub async fn capture_payment_intent(
        &self,
        payment_intent_id: &str,
    ) -> Result<PaymentIntent, BillingError> {
        let client = self.client()?;
        let id = parse_id::<PaymentIntentId>(payment_intent_id)?;
        let intent = PaymentIntent::capture(client, &id, Default::default()).await?;
        Ok(intent)
    }

    /// Cancel a payment intent.
    pub async fn cancel_payment_intent(
        &self,
        payment_intent_id: &str,
    ) -> Result<PaymentIntent, BillingError> {
        let client = self.client()?;
        let id = parse_id::<PaymentIntentId>(payment_intent_id)?;
        let intent = PaymentIntent::cancel(client, &id, Default::default()).await?;
        Ok(intent)
    }

    // -----------------------------------------------------------------------
    // Refunds
    // -----------------------------------------------------------------------

    /// Create a refund for a payment intent.
    ///
    /// Pass `None` for `amount` to refund the full remaining balance.
    pub async fn create_refund(
        &self,
        payment_intent_id: &str,
        amount: Option<i64>,
    ) -> Result<Refund, BillingError> {
        let client = self.client()?;
        let id = parse_id::<PaymentIntentId>(payment_intent_id)?;

        let mut params = CreateRefund::new();
        params.payment_intent = Some(id);
        params.amount = amount;

        let refund = Refund::create(client, params).await?;
        Ok(refund)
    }

    // -----------------------------------------------------------------------
    // Stripe Connect
    // -----------------------------------------------------------------------

    /// Create a Stripe Express connected account for a service provider.
    pub async fn create_connect_account(&self, email: &str) -> Result<Account, BillingError> {
        let client = self.client()?;

        let mut params = CreateAccount::new();
        params.type_ = Some(AccountType::Express);
        params.email = Some(email);

        let account = Account::create(client, params).await?;
        Ok(account)
    }

    /// Generate a single-use Connect Onboarding URL for an Express account.
    ///
    /// `return_url` is where Stripe redirects the user after they complete or
    /// exit onboarding; it is also used as the `refresh_url`.
    pub async fn generate_onboarding_link(
        &self,
        account_id: &str,
        return_url: &str,
    ) -> Result<String, BillingError> {
        let client = self.client()?;
        let account = parse_id::<AccountId>(account_id)?;

        let mut params = CreateAccountLink::new(account, AccountLinkType::AccountOnboarding);
        params.refresh_url = Some(return_url);
        params.return_url = Some(return_url);

        let link = AccountLink::create(client, params).await?;
        Ok(link.url)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a Stripe ID from a string, mapping parse errors to [`BillingError::InvalidId`].
fn parse_id<T>(raw: &str) -> Result<T, BillingError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    raw.parse::<T>()
        .map_err(|_| BillingError::InvalidId(raw.to_owned()))
}
