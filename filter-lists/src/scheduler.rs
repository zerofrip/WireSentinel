use crate::{FilterListEngine, FilterSubscription};
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

/// Periodic filter-list update scheduler.
pub struct FilterUpdateScheduler {
    scheduler: JobScheduler,
}

impl FilterUpdateScheduler {
    pub async fn new() -> Result<Self, String> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| format!("scheduler init: {e}"))?;
        Ok(Self { scheduler })
    }

    pub async fn schedule_subscription(
        &self,
        engine: Arc<FilterListEngine>,
        sub: &FilterSubscription,
    ) -> Result<Uuid, String> {
        let Some(interval_secs) = sub.update_interval_secs else {
            return Err(format!("subscription {} has no update interval", sub.id));
        };

        let cron = interval_to_cron(interval_secs);
        let sub_id = sub.id;

        let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
            let engine = Arc::clone(&engine);
            Box::pin(async move {
                if let Err(err) = engine.update_one(sub_id).await {
                    tracing::warn!(%sub_id, %err, "scheduled filter list update failed");
                }
            })
        })
        .map_err(|e| format!("create job for {sub_id}: {e}"))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| format!("add job for {sub_id}: {e}"))
    }

    pub async fn schedule_all(
        &self,
        engine: Arc<FilterListEngine>,
        subscriptions: &[FilterSubscription],
    ) -> Result<(), String> {
        for sub in subscriptions {
            if sub.enabled && sub.update_interval_secs.is_some() {
                self.schedule_subscription(Arc::clone(&engine), sub).await?;
            }
        }
        Ok(())
    }

    pub async fn start(&self) -> Result<(), String> {
        self.scheduler
            .start()
            .await
            .map_err(|e| format!("scheduler start: {e}"))
    }

    pub async fn shutdown(&mut self) -> Result<(), String> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| format!("scheduler shutdown: {e}"))
    }
}

fn interval_to_cron(secs: u32) -> String {
    let secs = secs.max(1);
    if secs < 60 {
        format!("*/{secs} * * * * *")
    } else if secs < 3600 {
        let mins = (secs / 60).max(1);
        format!("*/{mins} * * * *")
    } else {
        let hours = (secs / 3600).max(1);
        format!("0 */{hours} * * *")
    }
}
