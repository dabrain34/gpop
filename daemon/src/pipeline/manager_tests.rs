// manager_tests.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use super::manager::*;
use crate::event::{create_event_channel, PipelineEvent, PipelineState};

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

    // Pipeline IDs are sequential numeric strings starting from "0"
    assert!(!id.is_empty());
    assert!(id.chars().all(|c| c.is_ascii_digit()));
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

    // Clean up to stop the bus watcher task
    manager.shutdown().await;
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
            // Pipeline IDs are sequential numeric strings
            assert!(!pipeline_id.is_empty());
            assert!(pipeline_id.chars().all(|c| c.is_ascii_digit()));
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

#[tokio::test]
async fn test_update_pipeline() {
    init_gstreamer();
    let (tx, _rx) = create_event_channel();
    let manager = PipelineManager::new(tx);

    let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
    assert_eq!(manager.get_pipeline_description(&id).await.unwrap(), "fakesrc ! fakesink");

    // Update the pipeline with a new description
    manager.update_pipeline(&id, "videotestsrc ! fakesink").await.unwrap();

    // Verify the description changed but ID remains the same
    assert_eq!(manager.get_pipeline_description(&id).await.unwrap(), "videotestsrc ! fakesink");
    assert_eq!(manager.pipeline_count().await, 1);
}

#[tokio::test]
async fn test_update_pipeline_nonexistent() {
    init_gstreamer();
    let (tx, _rx) = create_event_channel();
    let manager = PipelineManager::new(tx);

    let result = manager.update_pipeline("nonexistent", "fakesrc ! fakesink").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_pipeline_invalid_description() {
    init_gstreamer();
    let (tx, _rx) = create_event_channel();
    let manager = PipelineManager::new(tx);

    let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();

    // Try to update with an invalid description
    let result = manager.update_pipeline(&id, "invalid_element_xyz ! fakesink").await;
    assert!(result.is_err());

    // Original pipeline should still be intact
    assert_eq!(manager.get_pipeline_description(&id).await.unwrap(), "fakesrc ! fakesink");
    assert_eq!(manager.pipeline_count().await, 1);
}

#[tokio::test]
async fn test_update_pipeline_emits_event() {
    init_gstreamer();
    let (tx, mut rx) = create_event_channel();
    let manager = PipelineManager::new(tx);

    let id = manager.add_pipeline("fakesrc ! fakesink").await.unwrap();
    let _ = rx.recv().await; // Consume PipelineAdded event from add

    manager.update_pipeline(&id, "videotestsrc ! fakesink").await.unwrap();

    let event = rx.recv().await.unwrap();
    match event {
        PipelineEvent::PipelineUpdated { pipeline_id, description } => {
            assert_eq!(pipeline_id, id);
            assert_eq!(description, "videotestsrc ! fakesink");
        }
        _ => panic!("Expected PipelineUpdated event after update"),
    }
}
