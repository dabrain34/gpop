use actix_files::Files;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use gpop_web::api::configure_routes;
use gpop_web::config::Config;
use gpop_web::job::{start_event_handler, start_progress_poller, JobManager};
use gpop_web::storage::StorageManager;
use gpop_web::ws::{handle_client_websocket, GpopConnection, ProgressBroadcaster};

/// WebSocket endpoint handler for browser clients
async fn ws_progress(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<Arc<ProgressBroadcaster>>,
) -> actix_web::Result<HttpResponse> {
    let (res, session, _msg_stream) = actix_ws::handle(&req, stream)?;

    // Spawn handler task
    let broadcaster = Arc::clone(broadcaster.get_ref());
    actix_web::rt::spawn(async move {
        handle_client_websocket(session, broadcaster).await;
    });

    Ok(res)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("gpop_web=debug".parse().unwrap())
                .add_directive("actix_web=info".parse().unwrap()),
        )
        .init();

    // Parse configuration
    let config = Config::parse();

    info!("gpop-web starting...");
    info!("Server: {}:{}", config.host, config.port);
    info!("gpop daemon: {}", config.gpop_url);
    info!("Data directory: {}", config.data_dir.display());

    // Connect to gpop-daemon
    let gpop = match GpopConnection::connect(&config.gpop_url).await {
        Ok(conn) => Arc::new(conn),
        Err(e) => {
            error!("Failed to connect to gpop-daemon at {}: {}", config.gpop_url, e);
            error!("Make sure gpop-daemon is running: cargo run --package gpop-daemon");
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                e.to_string(),
            ));
        }
    };

    info!("Connected to gpop-daemon");

    // Initialize storage
    let storage = match StorageManager::new(&config).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            error!("Failed to initialize storage: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ));
        }
    };

    // Create progress broadcaster
    let broadcaster = Arc::new(ProgressBroadcaster::new());

    // Create job manager
    let job_manager = Arc::new(JobManager::new(
        Arc::clone(&gpop),
        Arc::clone(&storage),
        Arc::clone(&broadcaster),
        config.clone(),
    ));

    // Start event handler
    let manager_clone = Arc::clone(&job_manager);
    let gpop_clone = Arc::clone(&gpop);
    tokio::spawn(async move {
        start_event_handler(manager_clone, gpop_clone).await;
    });

    // Start progress poller (every 500ms)
    let manager_clone = Arc::clone(&job_manager);
    tokio::spawn(async move {
        start_progress_poller(manager_clone, Duration::from_millis(500)).await;
    });

    // Get bind address
    let bind_addr = format!("{}:{}", config.host, config.port);

    // Determine static files path
    let static_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("../static")))
        .unwrap_or_else(|| std::path::PathBuf::from("./web/static"));

    info!("Static files: {}", static_path.display());

    // Start HTTP server
    info!("Starting HTTP server at http://{}", bind_addr);

    HttpServer::new(move || {
        App::new()
            // App data
            .app_data(web::Data::new(Arc::clone(&job_manager)))
            .app_data(web::Data::new(Arc::clone(&broadcaster)))
            .app_data(web::Data::new(config.clone()))
            // WebSocket endpoint
            .route("/ws/progress", web::get().to(ws_progress))
            // API routes
            .configure(configure_routes)
            // Static files (serve index.html for root)
            .service(Files::new("/", &static_path).index_file("index.html"))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
