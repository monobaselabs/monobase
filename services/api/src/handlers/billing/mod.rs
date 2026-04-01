mod repo;

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::error::{ApiError, FieldError};
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;
use crate::model::{PaginatedResult, PaginationMeta};

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItemBody {
    pub description: String,
    pub quantity: Option<i32>,
    /// Unit price in cents.
    pub unit_price: i32,
    /// Total line amount in cents (quantity * unit_price).
    pub amount: i32,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceBody {
    pub customer_id: String,
    pub merchant_id: String,
    pub merchant_account_id: Option<String>,
    /// Optional idempotency / correlation key (unique per invoice).
    pub context: Option<String>,
    /// Subtotal in cents.
    pub subtotal: i32,
    /// Tax in cents.
    pub tax: Option<i32>,
    /// Total in cents — must equal subtotal + tax.
    pub total: i32,
    /// ISO 4217 currency code, e.g. "USD".
    pub currency: Option<String>,
    /// "automatic" | "manual"
    pub payment_capture_method: Option<String>,
    pub payment_due_at: Option<String>,
    pub line_items: Option<Vec<LineItemBody>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInvoiceBody {
    pub customer_id: Option<String>,
    pub merchant_id: Option<String>,
    pub merchant_account_id: Option<String>,
    pub context: Option<String>,
    pub subtotal: Option<i32>,
    pub tax: Option<i32>,
    pub total: Option<i32>,
    pub currency: Option<String>,
    pub payment_capture_method: Option<String>,
    pub payment_due_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInvoicesQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub customer_id: Option<String>,
    pub merchant_id: Option<String>,
    pub merchant_account_id: Option<String>,
    /// Filter by invoice status: draft | open | paid | void | uncollectible
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayInvoiceBody {
    /// Optional payment method identifier forwarded to the Stripe service layer.
    #[allow(dead_code)]
    pub payment_method_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMerchantAccountBody {
    pub person_id: String,
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_create_invoice(body: &CreateInvoiceBody) -> Result<(), ApiError> {
    let mut field_errors: Vec<FieldError> = Vec::new();

    if body.customer_id.is_empty() {
        field_errors.push(FieldError {
            field: "customerId".to_string(),
            code: "REQUIRED".to_string(),
            message: "customerId is required".to_string(),
            value: None,
        });
    }

    if body.merchant_id.is_empty() {
        field_errors.push(FieldError {
            field: "merchantId".to_string(),
            code: "REQUIRED".to_string(),
            message: "merchantId is required".to_string(),
            value: None,
        });
    }

    if body.subtotal < 0 {
        field_errors.push(FieldError {
            field: "subtotal".to_string(),
            code: "RANGE".to_string(),
            message: "subtotal must be non-negative".to_string(),
            value: Some(serde_json::json!(body.subtotal)),
        });
    }

    if body.total < 0 {
        field_errors.push(FieldError {
            field: "total".to_string(),
            code: "RANGE".to_string(),
            message: "total must be non-negative".to_string(),
            value: Some(serde_json::json!(body.total)),
        });
    }

    let expected_total = body.subtotal + body.tax.unwrap_or(0);
    if body.total != expected_total {
        field_errors.push(FieldError {
            field: "total".to_string(),
            code: "INVALID".to_string(),
            message: format!(
                "total ({}) must equal subtotal + tax ({})",
                body.total, expected_total
            ),
            value: Some(serde_json::json!(body.total)),
        });
    }

    if let Some(method) = &body.payment_capture_method {
        if method != "automatic" && method != "manual" {
            field_errors.push(FieldError {
                field: "paymentCaptureMethod".to_string(),
                code: "INVALID".to_string(),
                message: "paymentCaptureMethod must be 'automatic' or 'manual'".to_string(),
                value: Some(serde_json::json!(method)),
            });
        }
    }

    if !field_errors.is_empty() {
        return Err(ApiError::validation("Validation failed", field_errors));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a string field from a JSON value row.
fn row_str<'a>(row: &'a serde_json::Value, key: &str) -> &'a str {
    row.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

// ---------------------------------------------------------------------------
// Invoice handlers
// ---------------------------------------------------------------------------

/// POST /billing/invoices
pub async fn create_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateInvoiceBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    validate_create_invoice(&body)?;

    // Generate a unique, human-readable invoice number: INV-<10-char nanoid>
    let invoice_number = format!("INV-{}", nanoid::nanoid!(10));

    let invoice =
        repo::create_invoice(&state.db, &user.id, &body, &invoice_number).await?;

    Ok((StatusCode::CREATED, Json(invoice)))
}

/// GET /billing/invoices
pub async fn list_invoices(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<PaginatedResult<serde_json::Value>>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100);

    let (data, total_count) = repo::list_invoices(&state.db, offset, limit, &query).await?;
    let count = data.len() as u64;

    Ok(Json(PaginatedResult {
        data,
        pagination: PaginationMeta::new(offset, limit, count, total_count),
    }))
}

/// GET /billing/invoices/:invoice
pub async fn get_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let invoice = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    Ok(Json(invoice))
}

/// PATCH /billing/invoices/:invoice
pub async fn update_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
    Json(body): Json<UpdateInvoiceBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Validate that the invoice exists
    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    // Only draft or open invoices may be updated
    let status = row_str(&existing, "status");
    if status != "draft" && status != "open" {
        return Err(ApiError::BusinessLogic {
            message: format!(
                "Cannot update invoice with status '{}'. Only draft or open invoices can be updated.",
                status
            ),
            code: "INVOICE_NOT_EDITABLE".to_string(),
        });
    }

    let updated = repo::update_invoice(&state.db, &invoice_id, &user.id, &body).await?;

    Ok(Json(updated))
}

/// DELETE /billing/invoices/:invoice
pub async fn delete_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    // Confirm existence before deletion
    repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    repo::delete_invoice(&state.db, &invoice_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /billing/invoices/:invoice/finalize
///
/// Transition: draft → open
pub async fn finalize_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    if row_str(&existing, "status") != "draft" {
        return Err(ApiError::BusinessLogic {
            message: "Only draft invoices can be finalized".to_string(),
            code: "INVOICE_NOT_DRAFT".to_string(),
        });
    }

    let updated = repo::transition_invoice_status(
        &state.db,
        &invoice_id,
        &user.id,
        "open",
        &[],
        None,
    )
    .await?;

    Ok(Json(updated))
}

/// POST /billing/invoices/:invoice/pay
///
/// Transition: open → paid (or open → requires_capture for manual capture)
pub async fn pay_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
    Json(_body): Json<PayInvoiceBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    if row_str(&existing, "status") != "open" {
        return Err(ApiError::BusinessLogic {
            message: "Only open invoices can be paid".to_string(),
            code: "INVOICE_NOT_OPEN".to_string(),
        });
    }

    // Determine capture method — manual capture stays in 'open' but sets
    // payment_status to 'requires_capture'; automatic goes straight to 'paid'.
    let capture_method = row_str(&existing, "payment_capture_method");

    let (new_status, new_payment_status) = if capture_method == "manual" {
        ("open", "requires_capture")
    } else {
        ("paid", "succeeded")
    };

    let extra: Vec<(&str, sea_orm::Value)> = if new_status == "paid" {
        vec![
            ("paid_at", "NOW()".into()),
            ("paid_by", user.id.clone().into()),
        ]
    } else {
        vec![
            ("authorized_at", "NOW()".into()),
            ("authorized_by", user.id.clone().into()),
        ]
    };

    let updated = repo::transition_invoice_status(
        &state.db,
        &invoice_id,
        &user.id,
        new_status,
        &extra,
        Some(new_payment_status),
    )
    .await?;

    Ok(Json(updated))
}

/// POST /billing/invoices/:invoice/capture
///
/// Transition: payment_status requires_capture → succeeded, invoice status → paid
pub async fn capture_invoice_payment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    if row_str(&existing, "payment_status") != "requires_capture" {
        return Err(ApiError::BusinessLogic {
            message: "Only invoices with payment_status 'requires_capture' can be captured"
                .to_string(),
            code: "INVALID_PAYMENT_STATUS".to_string(),
        });
    }

    let extra: Vec<(&str, sea_orm::Value)> = vec![
        ("paid_at", "NOW()".into()),
        ("paid_by", user.id.clone().into()),
    ];

    let updated = repo::transition_invoice_status(
        &state.db,
        &invoice_id,
        &user.id,
        "paid",
        &extra,
        Some("succeeded"),
    )
    .await?;

    Ok(Json(updated))
}

/// POST /billing/invoices/:invoice/void
///
/// Transition: draft | open → void
pub async fn void_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    let status = row_str(&existing, "status");
    if status != "draft" && status != "open" {
        return Err(ApiError::BusinessLogic {
            message: format!(
                "Only draft or open invoices can be voided (current status: '{}')",
                status
            ),
            code: "INVOICE_NOT_VOIDABLE".to_string(),
        });
    }

    let extra: Vec<(&str, sea_orm::Value)> = vec![
        ("voided_at", "NOW()".into()),
        ("voided_by", user.id.clone().into()),
    ];

    let updated = repo::transition_invoice_status(
        &state.db,
        &invoice_id,
        &user.id,
        "void",
        &extra,
        None,
    )
    .await?;

    Ok(Json(updated))
}

/// POST /billing/invoices/:invoice/mark-uncollectible
///
/// Transition: open → uncollectible
pub async fn mark_invoice_uncollectible(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    if row_str(&existing, "status") != "open" {
        return Err(ApiError::BusinessLogic {
            message: "Only open invoices can be marked uncollectible".to_string(),
            code: "INVOICE_NOT_OPEN".to_string(),
        });
    }

    let updated = repo::transition_invoice_status(
        &state.db,
        &invoice_id,
        &user.id,
        "uncollectible",
        &[],
        None,
    )
    .await?;

    Ok(Json(updated))
}

/// POST /billing/invoices/:invoice/refund
///
/// Marks a paid invoice as refunded by transitioning payment_status → "refunded".
/// The actual refund call to Stripe is handled by the service layer.
pub async fn refund_invoice_payment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let existing = repo::get_invoice(&state.db, &invoice_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Invoice not found".to_string(),
            resource_type: Some("Invoice".to_string()),
            resource: Some(invoice_id.clone()),
        })?;

    if row_str(&existing, "payment_status") != "succeeded" {
        return Err(ApiError::BusinessLogic {
            message: "Only invoices with payment_status 'succeeded' can be refunded".to_string(),
            code: "INVALID_PAYMENT_STATUS".to_string(),
        });
    }

    let updated = repo::transition_payment_status(
        &state.db,
        &invoice_id,
        &user.id,
        "refunded",
        &[],
    )
    .await?;

    Ok(Json(updated))
}

// ---------------------------------------------------------------------------
// Merchant account handlers
// ---------------------------------------------------------------------------

/// POST /billing/merchant-accounts
pub async fn create_merchant_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateMerchantAccountBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    if body.person_id.is_empty() {
        return Err(ApiError::validation(
            "Validation failed",
            vec![FieldError {
                field: "personId".to_string(),
                code: "REQUIRED".to_string(),
                message: "personId is required".to_string(),
                value: None,
            }],
        ));
    }

    let account = repo::create_merchant_account(&state.db, &user.id, &body).await?;

    Ok((StatusCode::CREATED, Json(account)))
}

/// GET /billing/merchant-accounts/:merchantAccount
pub async fn get_merchant_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(merchant_account_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let account = repo::get_merchant_account(&state.db, &merchant_account_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Merchant account not found".to_string(),
            resource_type: Some("MerchantAccount".to_string()),
            resource: Some(merchant_account_id.clone()),
        })?;

    Ok(Json(account))
}

/// POST /billing/merchant-accounts/:merchantAccount/dashboard
///
/// Returns the current merchant account state. Stripe Connect dashboard
/// link generation is deferred to the service layer.
pub async fn get_merchant_dashboard(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(merchant_account_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let account = repo::get_merchant_account(&state.db, &merchant_account_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Merchant account not found".to_string(),
            resource_type: Some("MerchantAccount".to_string()),
            resource: Some(merchant_account_id.clone()),
        })?;

    // Augment with a placeholder dashboard_url — real Stripe Connect login
    // link generation belongs in the service layer.
    let response = serde_json::json!({
        "merchantAccount": account,
        "dashboardUrl": null,
    });

    Ok(Json(response))
}

/// POST /billing/merchant-accounts/:merchantAccount/onboard
///
/// Returns the current merchant account state. Stripe Connect onboarding
/// link generation is deferred to the service layer.
pub async fn onboard_merchant_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(merchant_account_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

    let account = repo::get_merchant_account(&state.db, &merchant_account_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: "Merchant account not found".to_string(),
            resource_type: Some("MerchantAccount".to_string()),
            resource: Some(merchant_account_id.clone()),
        })?;

    // Augment with a placeholder onboarding_url — real Stripe Connect account
    // link generation belongs in the service layer.
    let response = serde_json::json!({
        "merchantAccount": account,
        "onboardingUrl": null,
    });

    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Webhook handlers
// ---------------------------------------------------------------------------

/// POST /billing/webhooks/stripe — public, no authentication.
///
/// Accepts the raw Stripe webhook payload and returns 200 OK.
/// Signature verification and event dispatching are handled in the service layer.
pub async fn handle_stripe_webhook(
    State(_state): State<Arc<AppState>>,
    _headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    // Log receipt for traceability without exposing raw PII
    tracing::info!(
        payload_bytes = body.len(),
        "Stripe webhook received"
    );

    Ok(StatusCode::OK)
}
