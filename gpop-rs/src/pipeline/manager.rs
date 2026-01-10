use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::error::{GpopError, Result};
use crate::event::{EventSender, PipelineEvent, PipelineState};
use crate::pipeline::parser::Pipeline;

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
        let id = format!("pipeline-{}", self.next_id.fetch_add(1, Ordering::SeqCst));

        let pipeline = Pipeline::new(id.clone(), description, self.event_tx.clone())?;
        let pipeline = Arc::new(Mutex::new(pipeline));

        Pipeline::start_bus_watch(Arc::clone(&pipeline));

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

    pub async fn shutdown(&self) {
        let mut pipelines = self.pipelines.write().await;

        for (id, pipeline) in pipelines.drain() {
            let p = pipeline.lock().await;
            let _ = p.stop();
            info!("Stopped pipeline '{}' during shutdown", id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::create_event_channel;

    fn init_gstreamer() {
        let _ = gstreamer::init();
    }

    #[tokio::test]
    async fn test_pipeline_manager_new() {
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        assert_eq!(manager.pipeline_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_pipeline() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        assert!(id.starts_with("pipeline-"));
        assert_eq!(manager.pipeline_count().await, 1);
    }

    #[tokio::test]
    async fn test_add_multiple_pipelines() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id1 = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let id2 = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let id3 = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(manager.pipeline_count().await, 3);
    }

    #[tokio::test]
    async fn test_add_invalid_pipeline() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let result = manager.add_pipeline("invalid_element_xyz ! fakesink").await;

        assert!(result.is_err());
        assert_eq!(manager.pipeline_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_pipeline() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        assert_eq!(manager.pipeline_count().await, 1);

        manager.remove_pipeline(&id).await.unwrap();
        assert_eq!(manager.pipeline_count().await, 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_pipeline() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let result = manager.remove_pipeline("nonexistent").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_pipeline_info() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let info = manager.get_pipeline_info(&id).await.unwrap();

        assert_eq!(info.id, id);
        assert_eq!(info.description, "fakesrc ! fakesink");
        assert_eq!(info.state, PipelineState::Null);
        assert!(!info.streaming);
    }

    #[tokio::test]
    async fn test_get_pipeline_description() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let desc = manager.get_pipeline_description(&id).await.unwrap();

        assert_eq!(desc, "fakesrc ! fakesink");
    }

    #[tokio::test]
    async fn test_list_pipelines() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        let list = manager.list_pipelines().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_pipelines_empty() {
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let list = manager.list_pipelines().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_set_state() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        manager.set_state(&id, PipelineState::Ready).await.unwrap();
        let info = manager.get_pipeline_info(&id).await.unwrap();
        assert_eq!(info.state, PipelineState::Ready);

        manager.set_state(&id, PipelineState::Null).await.unwrap();
    }

    #[tokio::test]
    async fn test_play_pause_stop() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        manager.play(&id).await.unwrap();
        // Give some time for state change
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        manager.pause(&id).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        manager.stop(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_dot() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let dot = manager.get_dot(&id, None).await.unwrap();

        assert!(dot.contains("digraph"));
    }

    #[tokio::test]
    async fn test_shutdown() {
        init_gstreamer();
        let (tx, _rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        assert_eq!(manager.pipeline_count().await, 2);

        manager.shutdown().await;
        assert_eq!(manager.pipeline_count().await, 0);
    }

    #[tokio::test]
    async fn test_events_emitted_on_add() {
        init_gstreamer();
        let (tx, mut rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            PipelineEvent::PipelineAdded { pipeline_id, description } => {
                assert!(pipeline_id.starts_with("pipeline-"));
                assert_eq!(description, "fakesrc ! fakesink");
            }
            _ => panic!("Expected PipelineAdded event"),
        }
    }

    #[tokio::test]
    async fn test_events_emitted_on_remove() {
        init_gstreamer();
        let (tx, mut rx) = create_event_channel();
        let manager = PipelineManager::new(tx);

        let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
        let _ = rx.recv().await; // Consume PipelineAdded event

        manager.remove_pipeline(&id).await.unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            PipelineEvent::PipelineRemoved { pipeline_id } => {
                assert_eq!(pipeline_id, id);
            }
            _ => panic!("Expected PipelineRemoved event"),
        }
    }
}
