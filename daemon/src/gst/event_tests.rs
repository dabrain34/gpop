// event_tests.rs
//
// Copyright 2026 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use super::event::*;

#[test]
fn test_pipeline_state_display() {
    assert_eq!(PipelineState::VoidPending.to_string(), "void_pending");
    assert_eq!(PipelineState::Null.to_string(), "null");
    assert_eq!(PipelineState::Ready.to_string(), "ready");
    assert_eq!(PipelineState::Paused.to_string(), "paused");
    assert_eq!(PipelineState::Playing.to_string(), "playing");
}

#[test]
fn test_pipeline_state_from_str() {
    assert_eq!(
        "void_pending".parse::<PipelineState>().unwrap(),
        PipelineState::VoidPending
    );
    assert_eq!(
        "voidpending".parse::<PipelineState>().unwrap(),
        PipelineState::VoidPending
    );
    assert_eq!(
        "null".parse::<PipelineState>().unwrap(),
        PipelineState::Null
    );
    assert_eq!(
        "ready".parse::<PipelineState>().unwrap(),
        PipelineState::Ready
    );
    assert_eq!(
        "paused".parse::<PipelineState>().unwrap(),
        PipelineState::Paused
    );
    assert_eq!(
        "playing".parse::<PipelineState>().unwrap(),
        PipelineState::Playing
    );

    // Case insensitive
    assert_eq!(
        "PLAYING".parse::<PipelineState>().unwrap(),
        PipelineState::Playing
    );
    assert_eq!(
        "Playing".parse::<PipelineState>().unwrap(),
        PipelineState::Playing
    );
}

#[test]
fn test_pipeline_state_from_str_invalid() {
    assert!("invalid".parse::<PipelineState>().is_err());
    assert!("".parse::<PipelineState>().is_err());
}

#[test]
fn test_pipeline_state_gstreamer_conversion() {
    assert_eq!(
        PipelineState::from(gstreamer::State::VoidPending),
        PipelineState::VoidPending
    );
    assert_eq!(
        PipelineState::from(gstreamer::State::Null),
        PipelineState::Null
    );
    assert_eq!(
        PipelineState::from(gstreamer::State::Ready),
        PipelineState::Ready
    );
    assert_eq!(
        PipelineState::from(gstreamer::State::Paused),
        PipelineState::Paused
    );
    assert_eq!(
        PipelineState::from(gstreamer::State::Playing),
        PipelineState::Playing
    );

    assert_eq!(
        gstreamer::State::from(PipelineState::VoidPending),
        gstreamer::State::VoidPending
    );
    assert_eq!(
        gstreamer::State::from(PipelineState::Null),
        gstreamer::State::Null
    );
    assert_eq!(
        gstreamer::State::from(PipelineState::Ready),
        gstreamer::State::Ready
    );
    assert_eq!(
        gstreamer::State::from(PipelineState::Paused),
        gstreamer::State::Paused
    );
    assert_eq!(
        gstreamer::State::from(PipelineState::Playing),
        gstreamer::State::Playing
    );
}

#[test]
fn test_pipeline_event_serialize_state_changed() {
    let event = PipelineEvent::StateChanged {
        pipeline_id: "pipeline-0".to_string(),
        old_state: PipelineState::Null,
        new_state: PipelineState::Playing,
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"state_changed\""));
    assert!(json.contains("\"pipeline_id\":\"pipeline-0\""));
    assert!(json.contains("\"old_state\":\"null\""));
    assert!(json.contains("\"new_state\":\"playing\""));
}

#[test]
fn test_pipeline_event_serialize_error() {
    let event = PipelineEvent::Error {
        pipeline_id: "pipeline-0".to_string(),
        message: "Test error".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"error\""));
    assert!(json.contains("\"message\":\"Test error\""));
}

#[test]
fn test_pipeline_event_serialize_unsupported() {
    let event = PipelineEvent::Unsupported {
        pipeline_id: "pipeline-0".to_string(),
        message: "No decoder available for type 'video/x-h265'".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"unsupported\""));
    assert!(json.contains("\"message\":\"No decoder available"));
}

#[test]
fn test_pipeline_event_serialize_eos() {
    let event = PipelineEvent::Eos {
        pipeline_id: "pipeline-0".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"eos\""));
}

#[test]
fn test_pipeline_event_serialize_pipeline_added() {
    let event = PipelineEvent::PipelineAdded {
        pipeline_id: "pipeline-0".to_string(),
        description: "videotestsrc ! autovideosink".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"pipeline_added\""));
    assert!(json.contains("\"description\":\"videotestsrc ! autovideosink\""));
}

#[test]
fn test_pipeline_event_serialize_pipeline_removed() {
    let event = PipelineEvent::PipelineRemoved {
        pipeline_id: "pipeline-0".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"pipeline_removed\""));
}

#[test]
fn test_pipeline_event_serialize_pipeline_updated() {
    let event = PipelineEvent::PipelineUpdated {
        pipeline_id: "pipeline-0".to_string(),
        description: "videotestsrc ! fakesink".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"pipeline_updated\""));
    assert!(json.contains("\"pipeline_id\":\"pipeline-0\""));
    assert!(json.contains("\"description\":\"videotestsrc ! fakesink\""));
}

#[test]
fn test_event_channel_creation() {
    let (tx, rx) = create_event_channel();

    // With a receiver present, send should succeed
    let result = tx.send(PipelineEvent::Eos {
        pipeline_id: "test".to_string(),
    });
    assert!(result.is_ok());

    // Drop receiver and send again - should fail (no receivers)
    drop(rx);
    let result = tx.send(PipelineEvent::Eos {
        pipeline_id: "test".to_string(),
    });
    assert!(result.is_err());
}

#[tokio::test]
async fn test_event_channel_send_receive() {
    let (tx, mut rx) = create_event_channel();

    let event = PipelineEvent::Eos {
        pipeline_id: "test".to_string(),
    };

    tx.send(event.clone()).unwrap();

    let received = rx.recv().await.unwrap();
    match received {
        PipelineEvent::Eos { pipeline_id } => {
            assert_eq!(pipeline_id, "test");
        }
        _ => panic!("Expected Eos event"),
    }
}
