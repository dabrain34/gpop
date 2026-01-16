// main.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[cfg(target_os = "linux")]
use gpop::dbus::{run_dbus_event_forwarder, DbusServer};
use gpop::event::create_event_channel;
use gpop::pipeline::PipelineManager;
use gpop::websocket::WebSocketServer;

#[derive(Parser, Debug)]
#[command(name = "gpop-rs")]
#[command(author = "Stéphane Cerveau")]
#[command(version)]
#[command(about = "GStreamer Prince of Parser - Pipeline management daemon")]
struct Args {
    /// WebSocket port
    #[arg(short, long, default_value_t = gpop::websocket::DEFAULT_WEBSOCKET_PORT)]
    port: u16,

    /// Bind address for WebSocket server
    #[arg(short, long, default_value = gpop::websocket::DEFAULT_BIND_ADDRESS)]
    bind: String,

    /// Initial pipeline(s) to create
    #[arg(short = 'P', long = "pipeline")]
    pipelines: Vec<String>,

    /// Disable DBus interface (Linux only)
    #[cfg(target_os = "linux")]
    #[arg(long)]
    no_dbus: bool,

    /// Disable WebSocket interface
    #[arg(long)]
    no_websocket: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("gpop=info".parse().unwrap())
                .add_directive("gpop_rs=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    // Validate that at least one interface is enabled
    #[cfg(target_os = "linux")]
    if args.no_dbus && args.no_websocket {
        error!("At least one interface (DBus or WebSocket) must be enabled");
        std::process::exit(1);
    }

    // Initialize GStreamer
    gstreamer::init()?;
    info!("GStreamer initialized");

    // Create event channel (receivers are created via event_tx.subscribe())
    let (event_tx, _) = create_event_channel();

    // Create pipeline manager
    let manager = Arc::new(PipelineManager::new(event_tx.clone()));

    // Create initial pipelines
    for desc in &args.pipelines {
        match manager.add_pipeline(desc).await {
            Ok(id) => info!("Created initial pipeline '{}': {}", id, desc),
            Err(e) => error!("Failed to create initial pipeline '{}': {}", desc, e),
        }
    }

    // Start DBus server (Linux only)
    #[cfg(target_os = "linux")]
    let dbus_server = if !args.no_dbus {
        match DbusServer::new(Arc::clone(&manager)).await {
            Ok(server) => {
                let server = Arc::new(server);

                // Start DBus event forwarder
                let dbus_server_clone = Arc::clone(&server);
                let dbus_event_rx = event_tx.subscribe();
                tokio::spawn(async move {
                    run_dbus_event_forwarder(dbus_server_clone, dbus_event_rx).await;
                });

                Some(server)
            }
            Err(e) => {
                error!("Failed to start DBus server: {}", e);
                if args.no_websocket {
                    std::process::exit(1);
                }
                None
            }
        }
    } else {
        info!("DBus interface disabled");
        None
    };

    // Start WebSocket server
    let ws_handle = if !args.no_websocket {
        let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse()?;
        let ws_server = WebSocketServer::new(addr, Arc::clone(&manager));
        let ws_event_rx = event_tx.subscribe();

        Some(tokio::spawn(async move {
            if let Err(e) = ws_server.run(ws_event_rx).await {
                error!("WebSocket server error: {}", e);
            }
        }))
    } else {
        info!("WebSocket interface disabled");
        None
    };

    // Wait for shutdown signal
    info!("gpop-rs started. Press Ctrl+C to stop.");

    #[cfg(unix)]
    {
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        info!("Received Ctrl+C");
    }

    // Graceful shutdown
    info!("Shutting down...");

    // Stop pipelines
    manager.shutdown().await;

    // Cancel WebSocket server
    if let Some(handle) = ws_handle {
        handle.abort();
    }

    // DBus connection will be dropped automatically (Linux only)
    #[cfg(target_os = "linux")]
    drop(dbus_server);

    info!("Shutdown complete");
    Ok(())
}
