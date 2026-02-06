// mod.rs
//
// Copyright 2026 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

pub mod manager;
pub mod pipeline;
pub mod protocol;
pub mod server;

pub use manager::ManagerInterface;
pub use server::WebSocketServer;

/// Maximum number of concurrent WebSocket clients
pub const MAX_CONCURRENT_CLIENTS: usize = 1000;

/// Buffer size for per-client message channels
pub const CLIENT_MESSAGE_BUFFER: usize = 256;

/// Default WebSocket port
pub const DEFAULT_WEBSOCKET_PORT: u16 = 9000;

/// Default bind address for WebSocket server
pub const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";

/// Default pipeline ID when not specified
pub const DEFAULT_PIPELINE_ID: &str = "0";

#[cfg(test)]
mod protocol_tests;
