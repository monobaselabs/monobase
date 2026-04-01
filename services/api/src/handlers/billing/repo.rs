use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use crate::error::ApiError;

use super::{
    CreateInvoiceBody, CreateMerchantAccountBody, ListInvoicesQuery, UpdateInvoiceBody,
};

// ---------------------------------------------------------------------------
// Invoice repos
// ---------------------------------------------------------------------------

/// Insert a new invoice and its line items in a single transaction.
pub async fn create_invoice(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateInvoiceBody,
    invoice_number: &str,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let metadata = body
        .metadata
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());

    let sql = r#"
        INSERT INTO "invoice" (
            id, invoice_number, customer_id, merchant_id, merchant_account_id,
            context, status, subtotal, tax, total, currency,
            payment_capture_method, payment_due_at, metadata,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, 'draft', $7, $8, $9, $10,
            $11, $12, $13::jsonb,
            $14, $14, NOW(), NOW(), 1
        )
        RETURNING *
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.clone().into(),
            invoice_number.into(),
            body.customer_id.clone().into(),
            body.merchant_id.clone().into(),
            body.merchant_account_id.clone().into(),
            body.context.clone().into(),
            body.subtotal.into(),
            body.tax.into(),
            body.total.into(),
            body.currency.clone().unwrap_or_else(|| "USD".to_string()).into(),
            body.payment_capture_method
                .clone()
                .unwrap_or_else(|| "automatic".to_string())
                .into(),
            body.payment_due_at.clone().into(),
            metadata.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505") {
            ApiError::conflict("An invoice with that context already exists")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create invoice"))?;

    // Insert line items if provided
    if let Some(ref line_items) = body.line_items {
        for item in line_items {
            let item_id = uuid::Uuid::new_v4().to_string();
            let item_metadata = item
                .metadata
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());

            let item_sql = r#"
                INSERT INTO "invoice_line_item" (
                    id, invoice_id, description, quantity, unit_price, amount,
                    metadata, created_by, updated_by, created_at, updated_at, version
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $8, NOW(), NOW(), 1)
            "#;

            db.execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                item_sql,
                vec![
                    item_id.into(),
                    id.clone().into(),
                    item.description.clone().into(),
                    item.quantity.unwrap_or(1).into(),
                    item.unit_price.into(),
                    item.amount.into(),
                    item_metadata.into(),
                    created_by.into(),
                ],
            ))
            .await
            .map_err(ApiError::Database)?;
        }
    }

    Ok(result)
}

/// List invoices with pagination and optional filters.
pub async fn list_invoices(
    db: &DatabaseConnection,
    offset: u64,
    limit: u64,
    query: &ListInvoicesQuery,
) -> Result<(Vec<serde_json::Value>, u64), ApiError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    if let Some(ref customer_id) = query.customer_id {
        conditions.push(format!("customer_id = ${}", idx));
        params.push(customer_id.clone().into());
        idx += 1;
    }

    if let Some(ref merchant_id) = query.merchant_id {
        conditions.push(format!("merchant_id = ${}", idx));
        params.push(merchant_id.clone().into());
        idx += 1;
    }

    if let Some(ref status) = query.status {
        conditions.push(format!("status = ${}", idx));
        params.push(status.clone().into());
        idx += 1;
    }

    if let Some(ref merchant_account_id) = query.merchant_account_id {
        conditions.push(format!("merchant_account_id = ${}", idx));
        params.push(merchant_account_id.clone().into());
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count
    let count_sql = format!(
        r#"SELECT COUNT(*) as count FROM "invoice" {}"#,
        where_clause
    );
    let count_result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &count_sql,
        params.clone(),
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    let total_count = count_result
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0);

    // Data
    let data_sql = format!(
        r#"SELECT * FROM "invoice" {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}"#,
        where_clause, idx, idx + 1,
    );

    let mut data_params = params;
    data_params.push((limit as i64).into());
    data_params.push((offset as i64).into());

    let data = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &data_sql,
        data_params,
    ))
    .all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok((data, total_count))
}

/// Get a single invoice by ID.
pub async fn get_invoice(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "invoice" WHERE id = $1"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}

/// Update editable fields on a draft/open invoice.
pub async fn update_invoice(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    body: &UpdateInvoiceBody,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec!["updated_at = NOW()".to_string(), "version = version + 1".to_string()];
    let mut params: Vec<sea_orm::Value> = Vec::new();
    let mut idx = 1u32;

    macro_rules! maybe_set {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = body.$field {
                set_clauses.push(format!("{} = ${}", $col, idx));
                params.push(val.clone().into());
                idx += 1;
            }
        };
    }

    maybe_set!(customer_id, "customer_id");
    maybe_set!(merchant_id, "merchant_id");
    maybe_set!(merchant_account_id, "merchant_account_id");
    maybe_set!(context, "context");
    maybe_set!(currency, "currency");
    maybe_set!(payment_capture_method, "payment_capture_method");
    maybe_set!(payment_due_at, "payment_due_at");

    if let Some(subtotal) = body.subtotal {
        set_clauses.push(format!("subtotal = ${}", idx));
        params.push(subtotal.into());
        idx += 1;
    }

    if let Some(tax) = body.tax {
        set_clauses.push(format!("tax = ${}", idx));
        params.push(tax.into());
        idx += 1;
    }

    if let Some(total) = body.total {
        set_clauses.push(format!("total = ${}", idx));
        params.push(total.into());
        idx += 1;
    }

    if let Some(ref val) = body.metadata {
        set_clauses.push(format!("metadata = ${}::jsonb", idx));
        params.push(val.to_string().into());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "invoice" SET {} WHERE id = ${} RETURNING *"#,
        set_clauses.join(", "),
        idx,
    );

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Invoice not found"))
}

/// Delete an invoice by ID.
pub async fn delete_invoice(db: &DatabaseConnection, id: &str) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "invoice" WHERE id = $1"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}

/// Transition invoice status, recording optional timestamp/actor fields.
///
/// `status_col` — the new value for the `status` column
/// `extra_sets` — additional `col = $N` fragments appended before the final `updated_by`
/// `extra_values` — values corresponding to those fragments (in order)
pub async fn transition_invoice_status(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    new_status: &str,
    extra_sets: &[(&str, sea_orm::Value)],
    payment_status: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec![
        "status = $1".to_string(),
        "updated_at = NOW()".to_string(),
        "version = version + 1".to_string(),
    ];
    let mut params: Vec<sea_orm::Value> = vec![new_status.into()];
    let mut idx = 2u32;

    if let Some(ps) = payment_status {
        set_clauses.push(format!("payment_status = ${}", idx));
        params.push(ps.into());
        idx += 1;
    }

    for (col, val) in extra_sets {
        set_clauses.push(format!("{} = ${}", col, idx));
        params.push(val.clone());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "invoice" SET {} WHERE id = ${} RETURNING *"#,
        set_clauses.join(", "),
        idx,
    );

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Invoice not found"))
}

/// Update only the payment_status column (and audit fields).
pub async fn transition_payment_status(
    db: &DatabaseConnection,
    id: &str,
    updated_by: &str,
    new_payment_status: &str,
    extra_sets: &[(&str, sea_orm::Value)],
) -> Result<serde_json::Value, ApiError> {
    let mut set_clauses = vec![
        "payment_status = $1".to_string(),
        "updated_at = NOW()".to_string(),
        "version = version + 1".to_string(),
    ];
    let mut params: Vec<sea_orm::Value> = vec![new_payment_status.into()];
    let mut idx = 2u32;

    for (col, val) in extra_sets {
        set_clauses.push(format!("{} = ${}", col, idx));
        params.push(val.clone());
        idx += 1;
    }

    set_clauses.push(format!("updated_by = ${}", idx));
    params.push(updated_by.into());
    idx += 1;

    params.push(id.into());

    let sql = format!(
        r#"UPDATE "invoice" SET {} WHERE id = ${} RETURNING *"#,
        set_clauses.join(", "),
        idx,
    );

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::not_found("Invoice not found"))
}

// ---------------------------------------------------------------------------
// Merchant account repos
// ---------------------------------------------------------------------------

/// Create a merchant account for a person (unique per person).
pub async fn create_merchant_account(
    db: &DatabaseConnection,
    created_by: &str,
    body: &CreateMerchantAccountBody,
) -> Result<serde_json::Value, ApiError> {
    let id = uuid::Uuid::new_v4().to_string();

    let metadata = body
        .metadata
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let sql = r#"
        INSERT INTO "merchant_account" (
            id, person_id, active, metadata,
            created_by, updated_by, created_at, updated_at, version
        )
        VALUES ($1, $2, true, $3::jsonb, $4, $4, NOW(), NOW(), 1)
        RETURNING *
    "#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            id.into(),
            body.person_id.clone().into(),
            metadata.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505") {
            ApiError::conflict("A merchant account already exists for this person")
        } else {
            ApiError::Database(e)
        }
    })?
    .ok_or_else(|| ApiError::internal("Failed to create merchant account"))
}

/// Get a merchant account by ID.
pub async fn get_merchant_account(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<serde_json::Value>, ApiError> {
    let sql = r#"SELECT * FROM "merchant_account" WHERE id = $1"#;

    JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)
}
