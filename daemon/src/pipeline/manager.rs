// manager.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::error::{GpopError, Result};
use crate::event::{EventSender, PipelineEvent, PipelineState};
use crate::pipeline::parser::Pipeline;
use super::SHUTDOWN_GRACE_PERIOD_MS;

pub struct PipelineInfo {
    pub id: String,
    pub description: String,
    pub state: PipelineState,
    pub streaming: bool,
}

pub struct PipelineManager {
    pipelines: RwLock<HashMap<String, Arc<Mutex<Pipeline>>>>,
    event_tx: EventSender,
    next_id: AtomicU32,
}

impl PipelineManager {
    pub fn new(event_tx: EventSender) -> Self {
        Self {
            pipelines: RwLock::new(HashMap::new()),
            event_tx,
            next_id: AtomicU32::new(0),
        }
    }

    pub async fn add_pipeline(&self, description: &str) -> Result<String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst).to_string();

        let pipeline = Pipeline::new(id.clone(), description)?;
        let pipeline = Arc::new(Mutex::new(pipeline));

        // Extract bus watch parameters synchronously to avoid race conditions
        let (bus, shutdown_flag) = {
            let p = pipeline.lock().await;
            let bus = p.bus().expect("Pipeline should have a bus");
            (bus, p.shutdown_flag())
        };

        // Start bus watcher and get the task handle
        let bus_task = Pipeline::start_bus_watch(
            bus,
            id.clone(),
            self.event_tx.clone(),
            shutdown_flag,
            Arc::clone(&pipeline),
        );

        // Store the task handle synchronously
        {
            let mut p = pipeline.lock().await;
            p.set_bus_task(bus_task);
        }

        {
            let mut pipelines = self.pipelines.write().await;
            pipelines.insert(id.clone(), pipeline);
        }

        info!("Added pipeline '{}': {}", id, description);

        if self
            .event_tx
            .send(PipelineEvent::PipelineAdded {
                pipeline_id: id.clone(),
                description: description.to_string(),
            })
            .is_err()
        {
            warn!("Failed to send PipelineAdded event: no receivers");
        }

        Ok(id)
    }

    pub async fn remove_pipeline(&self, id: &str) -> Result<()> {
        let mut pipelines = self.pipelines.write().await;

        if let Some(pipeline) = pipelines.remove(id) {
            {
                let p = pipeline.lock().await;
                p.stop()?;
            }

            info!("Removed pipeline '{}'", id);

            if self
                .event_tx
                .send(PipelineEvent::PipelineRemoved {
                    pipeline_id: id.to_string(),
                })
                .is_err()
            {
                warn!("Failed to send PipelineRemoved event: no receivers");
            }

            Ok(())
        } else {
            Err(GpopError::PipelineNotFound(id.to_string()))
        }
    }

    pub async fn get_pipeline(&self, id: &str) -> Result<Arc<Mutex<Pipeline>>> {
        let pipelines = self.pipelines.read().await;
        pipelines
            .get(id)
            .cloned()
            .ok_or_else(|| GpopError::PipelineNotFound(id.to_string()))
    }

    pub async fn get_pipeline_info(&self, id: &str) -> Result<PipelineInfo> {
        let pipeline = self.get_pipeline(id).await?;
        let p = pipeline.lock().await;

        Ok(PipelineInfo {
            id: p.id().to_string(),
            description: p.description().to_string(),
            state: p.state(),
            streaming: p.is_streaming(),
        })
    }

    pub async fn get_pipeline_description(&self, id: &str) -> Result<String> {
        let pipeline = self.get_pipeline(id).await?;
        let p = pipeline.lock().await;
        Ok(p.description().to_string())
    }

    pub async fn list_pipelines(&self) -> Vec<PipelineInfo> {
        // Collect pipeline references while holding the read lock briefly
        let pipeline_refs: Vec<Arc<Mutex<Pipeline>>> = {
            let pipelines = self.pipelines.read().await;
            pipelines.values().cloned().collect()
        };
        // Read lock is now released

        // Now iterate over pipelines without holding the outer lock
        let mut infos = Vec::with_capacity(pipeline_refs.len());
        for pipeline in pipeline_refs {
            let p = pipeline.lock().await;
            infos.push(PipelineInfo {
                id: p.id().to_string(),
                description: p.description().to_string(),
                state: p.state(),
                streaming: p.is_streaming(),
            });
        }

        infos
    }

    pub async fn pipeline_count(&self) -> usize {
        let pipelines = self.pipelines.read().await;
        pipelines.len()
    }

    pub async fn set_state(&self, id: &str, state: PipelineState) -> Result<()> {
        let pipeline = self.get_pipeline(id).await?;
        let p = pipeline.lock().await;
        p.set_state(state)
    }

    pub async fn play(&self, id: &str) -> Result<()> {
        self.set_state(id, PipelineState::Playing).await
    }

    pub async fn pause(&self, id: &str) -> Result<()> {
        self.set_state(id, PipelineState::Paused).await
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        self.set_state(id, PipelineState::Null).await
    }

    pub async fn get_dot(&self, id: &str, details: Option<&str>) -> Result<String> {
        let pipeline = self.get_pipeline(id).await?;
        let p = pipeline.lock().await;
        Ok(p.get_dot(details))
    }

    /// Get the current position and duration of a pipeline in nanoseconds.
    pub async fn get_position(&self, id: &str) -> Result<(Option<u64>, Option<u64>)> {
        let pipeline = self.get_pipeline(id).await?;
        let p = pipeline.lock().await;
        Ok(p.get_position())
    }

    pub async fn shutdown(&self) {
        let pipelines_to_stop: Vec<_> = {
            let mut pipelines = self.pipelines.write().await;
            pipelines.drain().collect()
        };

        for (id, pipeline) in pipelines_to_stop {
            // Signal shutdown first (doesn't require lock as it uses atomic)
            {
                let p = pipeline.lock().await;
                p.signal_shutdown();
            }
            // Give bus watcher time to see the shutdown flag
            tokio::time::sleep(tokio::time::Duration::from_millis(SHUTDOWN_GRACE_PERIOD_MS)).await;
            // Now stop the pipeline
            {
                let p = pipeline.lock().await;
                let _ = p.stop();
            }
            info!("Stopped pipeline '{}' during shutdown", id);
        }
    }
}
