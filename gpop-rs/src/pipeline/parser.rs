use gstreamer::prelude::*;
use gstreamer::{self as gst, DebugGraphDetails};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::error::{GpopError, Result};
use crate::event::{EventSender, PipelineEvent, PipelineState};

/// Maximum length for pipeline descriptions to prevent memory exhaustion
pub const MAX_PIPELINE_DESCRIPTION_LENGTH: usize = 64 * 1024; // 64KB

/// Timeout for state changes in seconds
pub const STATE_CHANGE_TIMEOUT_SECS: u64 = 30;

pub struct Pipeline {
    id: String,
    description: String,
    pipeline: gst::Pipeline,
    event_tx: EventSender,
    bus_task: Option<tokio::task::JoinHandle<()>>,
    /// Flag to signal the bus watcher to stop
    shutdown_flag: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new(id: String, description: &str, event_tx: EventSender) -> Result<Self> {
        // Validate description length
        if description.len() > MAX_PIPELINE_DESCRIPTION_LENGTH {
            return Err(GpopError::InvalidPipeline(format!(
                "Pipeline description too long: {} bytes (max: {} bytes)",
                description.len(),
                MAX_PIPELINE_DESCRIPTION_LENGTH
            )));
        }

        // Note: gst::init() should be called once at startup, not here
        // This is kept for safety in case Pipeline::new is called without prior initialization
        let _ = gst::init();

        let pipeline = gst::parse::launch(description)
            .map_err(|e| GpopError::InvalidPipeline(e.to_string()))?
            .downcast::<gst::Pipeline>()
            .map_err(|_| GpopError::InvalidPipeline("Not a pipeline".to_string()))?;

        info!("Created pipeline '{}': {}", id, description);

        Ok(Self {
            id,
            description: description.to_string(),
            pipeline,
            event_tx,
            bus_task: None,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn start_bus_watch(pipeline: Arc<Mutex<Self>>) {
        let pipeline_clone = Arc::clone(&pipeline);

        let task = tokio::spawn(async move {
            let (bus, id, event_tx, shutdown_flag) = {
                let p = pipeline_clone.lock().await;
                let bus = p.pipeline.bus().expect("Pipeline should have a bus");
                (
                    bus,
                    p.id.clone(),
                    p.event_tx.clone(),
                    Arc::clone(&p.shutdown_flag),
                )
            };

            loop {
                // Check shutdown flag first
                if shutdown_flag.load(Ordering::Relaxed) {
                    debug!("Bus watcher for pipeline '{}' received shutdown signal", id);
                    break;
                }

                let msg = {
                    let timeout = gst::ClockTime::from_mseconds(100);
                    bus.timed_pop(timeout)
                };

                if let Some(msg) = msg {
                    match msg.view() {
                        gst::MessageView::Error(err) => {
                            let error_msg = format!(
                                "{}: {}",
                                err.error(),
                                err.debug().unwrap_or_default()
                            );
                            error!("Pipeline '{}' error: {}", id, error_msg);
                            if event_tx
                                .send(PipelineEvent::Error {
                                    pipeline_id: id.clone(),
                                    message: error_msg,
                                })
                                .is_err()
                            {
                                warn!("Failed to send error event for pipeline '{}': no receivers", id);
                            }
                        }
                        gst::MessageView::Warning(warning) => {
                            warn!(
                                "Pipeline '{}' warning: {}",
                                id,
                                warning.debug().unwrap_or_default()
                            );
                        }
                        gst::MessageView::Eos(_) => {
                            info!("Pipeline '{}' reached end of stream", id);
                            if event_tx
                                .send(PipelineEvent::Eos {
                                    pipeline_id: id.clone(),
                                })
                                .is_err()
                            {
                                warn!("Failed to send EOS event for pipeline '{}': no receivers", id);
                            }
                        }
                        gst::MessageView::StateChanged(state_changed) => {
                            if let Some(src) = msg.src() {
                                let p = pipeline_clone.lock().await;
                                if src == p.pipeline.upcast_ref::<gst::Object>() {
                                    let old = PipelineState::from(state_changed.old());
                                    let new = PipelineState::from(state_changed.current());
                                    debug!(
                                        "Pipeline '{}' state changed: {} -> {}",
                                        id, old, new
                                    );
                                    if event_tx
                                        .send(PipelineEvent::StateChanged {
                                            pipeline_id: id.clone(),
                                            old_state: old,
                                            new_state: new,
                                        })
                                        .is_err()
                                    {
                                        warn!(
                                            "Failed to send state change event for pipeline '{}': no receivers",
                                            id
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Small yield to prevent busy-looping
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            debug!("Bus watcher for pipeline '{}' stopped", id);
        });

        // Store the task handle
        tokio::spawn(async move {
            let mut p = pipeline.lock().await;
            p.bus_task = Some(task);
        });
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn state(&self) -> PipelineState {
        // state() returns (Result<StateChangeSuccess, StateChangeError>, State, State)
        let (_result, current, _pending) = self.pipeline.state(gst::ClockTime::ZERO);
        PipelineState::from(current)
    }

    pub fn is_streaming(&self) -> bool {
        matches!(self.state(), PipelineState::Playing)
    }

    pub fn set_state(&self, state: PipelineState) -> Result<()> {
        let gst_state: gst::State = state.into();
        self.pipeline
            .set_state(gst_state)
            .map_err(|e| GpopError::StateChangeFailed(e.to_string()))?;

        // Wait for state change with timeout
        let timeout = gst::ClockTime::from_seconds(STATE_CHANGE_TIMEOUT_SECS);
        let (result, current, _pending) = self.pipeline.state(timeout);

        match result {
            Ok(success) => {
                match success {
                    gst::StateChangeSuccess::Success | gst::StateChangeSuccess::NoPreroll => {
                        info!("Pipeline '{}' state set to {}", self.id, state);
                        Ok(())
                    }
                    gst::StateChangeSuccess::Async => {
                        // State change is still in progress but was accepted
                        info!(
                            "Pipeline '{}' state change to {} in progress (current: {:?})",
                            self.id, state, current
                        );
                        Ok(())
                    }
                }
            }
            Err(_) => Err(GpopError::StateChangeFailed(format!(
                "Failed to change state to {} for pipeline '{}'",
                state, self.id
            ))),
        }
    }

    pub fn play(&self) -> Result<()> {
        self.set_state(PipelineState::Playing)
    }

    pub fn pause(&self) -> Result<()> {
        self.set_state(PipelineState::Paused)
    }

    pub fn stop(&self) -> Result<()> {
        self.set_state(PipelineState::Null)
    }

    pub fn get_dot(&self, details: Option<&str>) -> String {
        let detail_flags = match details {
            Some("media") => DebugGraphDetails::MEDIA_TYPE,
            Some("caps") => DebugGraphDetails::CAPS_DETAILS,
            Some("non-default") => DebugGraphDetails::NON_DEFAULT_PARAMS,
            Some("states") => DebugGraphDetails::STATES,
            Some("all") | None => DebugGraphDetails::all(),
            Some(_) => DebugGraphDetails::all(),
        };

        self.pipeline.debug_to_dot_data(detail_flags).to_string()
    }

    /// Signal the bus watcher to stop
    pub fn signal_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        debug!("Dropping pipeline '{}'", self.id);

        // Signal bus watcher to stop
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // Set pipeline to Null state
        let _ = self.pipeline.set_state(gst::State::Null);

        // Abort the bus task if it exists
        if let Some(task) = self.bus_task.take() {
            task.abort();
        }
    }
}
