#[cfg(target_os = "linux")]
pub mod dbus;
pub mod error;
pub mod event;
pub mod pipeline;
pub mod websocket;

pub use error::{GpopError, Result};
pub use event::{create_event_channel, PipelineEvent, PipelineState};
pub use pipeline::{Pipeline, PipelineInfo, PipelineManager};
