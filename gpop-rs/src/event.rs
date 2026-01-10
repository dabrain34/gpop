use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    /// Pipeline is in void/pending state (transitioning)
    VoidPending,
    Null,
    Ready,
    Paused,
    Playing,
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineState::VoidPending => write!(f, "void_pending"),
            PipelineState::Null => write!(f, "null"),
            PipelineState::Ready => write!(f, "ready"),
            PipelineState::Paused => write!(f, "paused"),
            PipelineState::Playing => write!(f, "playing"),
        }
    }
}

impl std::str::FromStr for PipelineState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "void_pending" | "voidpending" => Ok(PipelineState::VoidPending),
            "null" => Ok(PipelineState::Null),
            "ready" => Ok(PipelineState::Ready),
            "paused" => Ok(PipelineState::Paused),
            "playing" => Ok(PipelineState::Playing),
            _ => Err(format!("Invalid state: {}", s)),
        }
    }
}

impl From<gstreamer::State> for PipelineState {
    fn from(state: gstreamer::State) -> Self {
        match state {
            gstreamer::State::VoidPending => PipelineState::VoidPending,
            gstreamer::State::Null => PipelineState::Null,
            gstreamer::State::Ready => PipelineState::Ready,
            gstreamer::State::Paused => PipelineState::Paused,
            gstreamer::State::Playing => PipelineState::Playing,
        }
    }
}

impl From<PipelineState> for gstreamer::State {
    fn from(state: PipelineState) -> Self {
        match state {
            PipelineState::VoidPending => gstreamer::State::VoidPending,
            PipelineState::Null => gstreamer::State::Null,
            PipelineState::Ready => gstreamer::State::Ready,
            PipelineState::Paused => gstreamer::State::Paused,
            PipelineState::Playing => gstreamer::State::Playing,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum PipelineEvent {
    #[serde(rename = "state_changed")]
    StateChanged {
        pipeline_id: String,
        old_state: PipelineState,
        new_state: PipelineState,
    },
    #[serde(rename = "error")]
    Error { pipeline_id: String, message: String },
    #[serde(rename = "eos")]
    Eos { pipeline_id: String },
    #[serde(rename = "pipeline_added")]
    PipelineAdded {
        pipeline_id: String,
        description: String,
    },
    #[serde(rename = "pipeline_removed")]
    PipelineRemoved { pipeline_id: String },
}

pub type EventSender = tokio::sync::broadcast::Sender<PipelineEvent>;
pub type EventReceiver = tokio::sync::broadcast::Receiver<PipelineEvent>;

pub fn create_event_channel() -> (EventSender, EventReceiver) {
    tokio::sync::broadcast::channel(256)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!("void_pending".parse::<PipelineState>().unwrap(), PipelineState::VoidPending);
        assert_eq!("voidpending".parse::<PipelineState>().unwrap(), PipelineState::VoidPending);
        assert_eq!("null".parse::<PipelineState>().unwrap(), PipelineState::Null);
        assert_eq!("ready".parse::<PipelineState>().unwrap(), PipelineState::Ready);
        assert_eq!("paused".parse::<PipelineState>().unwrap(), PipelineState::Paused);
        assert_eq!("playing".parse::<PipelineState>().unwrap(), PipelineState::Playing);

        // Case insensitive
        assert_eq!("PLAYING".parse::<PipelineState>().unwrap(), PipelineState::Playing);
        assert_eq!("Playing".parse::<PipelineState>().unwrap(), PipelineState::Playing);
    }

    #[test]
    fn test_pipeline_state_from_str_invalid() {
        assert!("invalid".parse::<PipelineState>().is_err());
        assert!("".parse::<PipelineState>().is_err());
    }

    #[test]
    fn test_pipeline_state_gstreamer_conversion() {
        assert_eq!(PipelineState::from(gstreamer::State::VoidPending), PipelineState::VoidPending);
        assert_eq!(PipelineState::from(gstreamer::State::Null), PipelineState::Null);
        assert_eq!(PipelineState::from(gstreamer::State::Ready), PipelineState::Ready);
        assert_eq!(PipelineState::from(gstreamer::State::Paused), PipelineState::Paused);
        assert_eq!(PipelineState::from(gstreamer::State::Playing), PipelineState::Playing);

        assert_eq!(gstreamer::State::from(PipelineState::VoidPending), gstreamer::State::VoidPending);
        assert_eq!(gstreamer::State::from(PipelineState::Null), gstreamer::State::Null);
        assert_eq!(gstreamer::State::from(PipelineState::Ready), gstreamer::State::Ready);
        assert_eq!(gstreamer::State::from(PipelineState::Paused), gstreamer::State::Paused);
        assert_eq!(gstreamer::State::from(PipelineState::Playing), gstreamer::State::Playing);
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
}
