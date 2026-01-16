// websocket_integration.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

//! Integration tests for WebSocket server bounded channels and connection limits

use gpop::websocket::{CLIENT_MESSAGE_BUFFER, MAX_CONCURRENT_CLIENTS};

#[test]
fn test_client_message_buffer_is_bounded() {
    // Verify the constant is set to 256 (matching event channel buffer)
    assert_eq!(
        CLIENT_MESSAGE_BUFFER, 256,
        "CLIENT_MESSAGE_BUFFER should be 256 to match event channel buffer size"
    );
}

#[test]
fn test_max_concurrent_clients_is_reasonable() {
    // Verify the constant is set to a reasonable value for production use
    assert_eq!(
        MAX_CONCURRENT_CLIENTS, 1000,
        "MAX_CONCURRENT_CLIENTS should be 1000"
    );
}

#[test]
fn test_constants_are_public() {
    // This test verifies that the constants are exported and can be used
    // by downstream code if needed
    let _ = CLIENT_MESSAGE_BUFFER;
    let _ = MAX_CONCURRENT_CLIENTS;
}
