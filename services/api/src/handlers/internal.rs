use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde_json::{json, Value};
use std::sync::Arc;

use super::AppState;

/// GET /livez - Liveness probe
pub async fn livez() -> Json<Value> {
    Json(json!({ "status": "pass" }))
}

/// GET /readyz - Readiness probe
pub async fn readyz(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    // Check database connectivity
    let db_status = match state.db.execute_unprepared("SELECT 1").await {
        Ok(_) => "pass",
        Err(_) => "fail",
    };

    let status = if db_status == "pass" { "pass" } else { "fail" };

    Ok(Json(json!({
        "status": status,
        "checks": {
            "database:connectivity": [{
                "status": db_status,
                "componentType": "datastore"
            }]
        }
    })))
}

/// GET /.well-known/jwks.json - JWKS endpoint
pub async fn jwks() -> Json<Value> {
    // Return a minimal JWKS with a generated key
    // In production this would use real keys from config
    Json(json!({
        "keys": [{
            "kty": "EC",
            "kid": "monobase-api-key-1",
            "alg": "ES256",
            "use": "sig",
            "crv": "P-256",
            "x": "y8EI5bP6iOIuyWwOESEsnCZ_GtVy45Fw_ethp3M9nNY",
            "y": "pGDWP8cunX8_jc_to0xNUTxAef1BXOJTYfZ7_OGSElA"
        }]
    }))
}

/// POST /internal/test-reset - Reset database for test isolation
pub async fn test_reset(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    // Truncate all tables for test isolation
    let result = state.db.execute_unprepared(
        r#"
        DO $$ DECLARE r RECORD;
        BEGIN
          FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') LOOP
            EXECUTE 'TRUNCATE TABLE "' || r.tablename || '" CASCADE';
          END LOOP;
        END $$;
        "#
    ).await;

    match result {
        Ok(_) => {
            // Re-create auth tables after truncation (they were truncated, not dropped)
            // Auth tables still exist, just need data — no DDL needed
            Ok(Json(json!({ "status": "ok" })))
        }
        Err(e) => {
            tracing::error!("Failed to reset database: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
