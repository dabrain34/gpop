// mod.rs
//
// Copyright 2026 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

pub mod event;
pub mod manager;
pub mod pipeline;

pub use event::{create_event_channel, EventReceiver, EventSender, PipelineEvent, PipelineState};
pub use manager::{PipelineInfo, PipelineManager};
pub use pipeline::Pipeline;

/// Grace period in milliseconds to wait for bus watcher to shutdown
pub const SHUTDOWN_GRACE_PERIOD_MS: u64 = 150;

#[cfg(test)]
mod event_tests;

#[cfg(test)]
mod manager_tests;

#[cfg(test)]
mod pipeline_tests;
