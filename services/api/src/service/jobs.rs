//! Background job scheduler.
//! Polls pg-boss compatible tables for pending jobs.

use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};
use std::sync::Arc;
use tokio::sync::Mutex;

type JobHandler = Arc<dyn Fn(serde_json::Value) -> tokio::task::JoinHandle<()> + Send + Sync>;

pub struct JobScheduler {
    db: DatabaseConnection,
    handlers: Arc<Mutex<std::collections::HashMap<String, JobHandler>>>,
    running: Arc<Mutex<bool>>,
}

impl JobScheduler {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            handlers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a job handler by name.
    pub async fn register<F>(&self, name: &str, handler: F)
    where
        F: Fn(serde_json::Value) -> tokio::task::JoinHandle<()> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().await;
        handlers.insert(name.to_string(), Arc::new(handler));
    }

    /// Trigger a job manually.
    pub async fn trigger(&self, name: &str, data: serde_json::Value) -> Result<String, String> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let data_str = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());

        // Insert into pgboss.job table
        let sql = r#"
            INSERT INTO pgboss.job (id, name, data, state, created_on, started_on)
            VALUES ($1, $2, $3::jsonb, 'created', NOW(), NULL)
        "#;

        self.db
            .execute(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                vec![job_id.clone().into(), name.into(), data_str.into()],
            ))
            .await
            .map_err(|e| format!("Failed to trigger job: {}", e))?;

        tracing::info!(job_id = %job_id, job_name = %name, "Job triggered");
        Ok(job_id)
    }

    /// Start the job polling loop.
    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let db = self.db.clone();
        let handlers = self.handlers.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            tracing::info!("Job scheduler started");
            loop {
                {
                    let is_running = running.lock().await;
                    if !*is_running {
                        break;
                    }
                }

                // Poll for pending jobs
                let sql = r#"
                    UPDATE pgboss.job
                    SET state = 'active', started_on = NOW()
                    WHERE id = (
                        SELECT id FROM pgboss.job
                        WHERE state = 'created'
                        ORDER BY created_on ASC
                        LIMIT 1
                        FOR UPDATE SKIP LOCKED
                    )
                    RETURNING id, name, data
                "#;

                match JsonValue::find_by_statement(Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    sql,
                    vec![],
                ))
                .one(&db)
                .await
                {
                    Ok(Some(job)) => {
                        let job_id = job.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let job_name = job.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let job_data = job.get("data").cloned().unwrap_or(serde_json::Value::Null);

                        tracing::info!(job_id = %job_id, job_name = %job_name, "Processing job");

                        let handlers_guard = handlers.lock().await;
                        if let Some(handler) = handlers_guard.get(&job_name) {
                            let handle = handler(job_data);
                            let db_clone = db.clone();
                            let job_id_clone = job_id.clone();

                            tokio::spawn(async move {
                                match handle.await {
                                    Ok(_) => {
                                        let _ = db_clone.execute(Statement::from_sql_and_values(
                                            sea_orm::DatabaseBackend::Postgres,
                                            "UPDATE pgboss.job SET state = 'completed', completed_on = NOW() WHERE id = $1",
                                            vec![job_id_clone.into()],
                                        )).await;
                                    }
                                    Err(e) => {
                                        tracing::error!(job_id = %job_id, error = %e, "Job failed");
                                        let _ = db_clone.execute(Statement::from_sql_and_values(
                                            sea_orm::DatabaseBackend::Postgres,
                                            "UPDATE pgboss.job SET state = 'failed', completed_on = NOW() WHERE id = $1",
                                            vec![job_id.into()],
                                        )).await;
                                    }
                                }
                            });
                        } else {
                            tracing::warn!(job_name = %job_name, "No handler registered for job");
                            let _ = db.execute(Statement::from_sql_and_values(
                                sea_orm::DatabaseBackend::Postgres,
                                "UPDATE pgboss.job SET state = 'failed', completed_on = NOW() WHERE id = $1",
                                vec![job_id.into()],
                            )).await;
                        }
                    }
                    Ok(None) => {
                        // No jobs available, wait before polling again
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        // pg-boss tables might not exist yet — that's ok
                        tracing::trace!(error = %e, "Job poll error (pgboss tables may not exist)");
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }
                }
            }
            tracing::info!("Job scheduler stopped");
        });
    }

    /// Stop the job polling loop.
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    /// Get job queue health.
    pub async fn health(&self) -> Result<serde_json::Value, String> {
        let sql = r#"
            SELECT
                COUNT(*) FILTER (WHERE state = 'created') as pending,
                COUNT(*) FILTER (WHERE state = 'active') as active,
                COUNT(*) FILTER (WHERE state = 'completed') as completed,
                COUNT(*) FILTER (WHERE state = 'failed') as failed
            FROM pgboss.job
        "#;

        let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![],
        ))
        .one(&self.db)
        .await
        .map_err(|e| format!("Failed to get job health: {}", e))?;

        Ok(result.unwrap_or(serde_json::json!({})))
    }
}
