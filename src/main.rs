use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use bytes::Bytes;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::time::{sleep, Duration};
use prometheus::{Encoder, TextEncoder, IntCounter, Registry};
use config::Config;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct LogEntry {
    timestamp: String,
    service: String,
    level: String,
    message: String,
}

// Shared in-memory storage
type LogDb = Arc<Mutex<Vec<LogEntry>>>;
type Broadcaster = Arc<broadcast::Sender<String>>;

// Load configuration
#[derive(Debug, Deserialize, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize, Clone)]
struct LoggingConfig {
    file_path: String,
    max_memory_logs: usize,
    persist_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct AppConfig {
    server: ServerConfig,
    logging: LoggingConfig,
}

fn load_config() -> AppConfig {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}

// POST /logs
async fn post_log(
    db: web::Data<LogDb>,
    bcast: web::Data<Broadcaster>,
    log: web::Json<LogEntry>,
    total_logs: web::Data<IntCounter>,
) -> impl Responder {
    let mut entry = log.into_inner();

    if entry.timestamp.trim().is_empty() {
        entry.timestamp = Utc::now().to_rfc3339();
    }

    {
        let mut db_lock = db.lock().unwrap();
        db_lock.push(entry.clone());

        // Keep memory bounded safely
        let len = db_lock.len();
        if len > 50_000 {
            db_lock.drain(0..len - 50_000);
        }
    }

    // Broadcast to SSE subscribers
    if let Ok(payload) = serde_json::to_string(&entry) {
        let _ = bcast.send(payload);
    }

    total_logs.inc(); // increment Prometheus counter
    HttpResponse::Ok().body("Log added")
}

// GET /logs?service=...&level=...
async fn get_logs(
    db: web::Data<LogDb>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let db_lock = db.lock().unwrap();
    let mut filtered: Vec<LogEntry> = db_lock.clone();

    if let Some(service) = query.get("service") {
        filtered.retain(|log| log.service == *service);
    }
    if let Some(level) = query.get("level") {
        filtered.retain(|log| log.level == *level);
    }

    HttpResponse::Ok().json(filtered)
}

// GET /logs/stats
async fn get_stats(db: web::Data<LogDb>) -> impl Responder {
    let db_lock = db.lock().unwrap();
    use std::collections::HashMap;

    let mut by_level: HashMap<String, usize> = HashMap::new();
    let mut by_service: HashMap<String, usize> = HashMap::new();

    for log in db_lock.iter() {
        *by_level.entry(log.level.clone()).or_insert(0) += 1;
        *by_service.entry(log.service.clone()).or_insert(0) += 1;
    }

    let stats = serde_json::json!({
        "by_level": by_level,
        "by_service": by_service
    });

    HttpResponse::Ok().json(stats)
}

// Serve index.html
async fn index() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("static/index.html")?)
}

// SSE: /logs/stream
async fn logs_stream(bcast: web::Data<Broadcaster>) -> HttpResponse {
    let rx = bcast.subscribe();

    let stream = futures::stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(msg) => {
                let sse_frame = format!("data: {}\n\n", msg);
                Some((Ok::<Bytes, std::io::Error>(Bytes::from(sse_frame)), rx))
            }
            Err(_) => None,
        }
    });

    HttpResponse::Ok()
        .append_header(("Content-Type", "text/event-stream"))
        .append_header(("Cache-Control", "no-cache"))
        .streaming(stream)
}

// Metrics endpoint
async fn metrics(registry: web::Data<Registry>) -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    HttpResponse::Ok()
        .append_header(("Content-Type", encoder.format_type()))
        .body(buffer)
}

// Async persistence task
async fn persist_logs(db: LogDb, cfg: LoggingConfig) {
    loop {
        sleep(Duration::from_secs(cfg.persist_interval_secs)).await;

        let logs = {
            let db_lock = db.lock().unwrap();
            db_lock.clone()
        };

        if !logs.is_empty() {
            let json = serde_json::to_string(&logs).unwrap();
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&cfg.file_path)
                .unwrap();
            file.write_all(json.as_bytes()).unwrap();
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cfg = load_config();

    let db: LogDb = Arc::new(Mutex::new(Vec::new()));
    let bcast: Broadcaster = Arc::new(broadcast::channel(100).0);

    // Prometheus metrics
    let registry = Registry::new();
    let total_logs = IntCounter::new("total_logs", "Total number of logs received").unwrap();
    registry.register(Box::new(total_logs.clone())).unwrap();

    // Spawn persistence task
    let persist_db = db.clone();
    let persist_cfg = cfg.logging.clone();
    tokio::spawn(async move { persist_logs(persist_db, persist_cfg).await });

    println!("Server running at http://{}:{}/", cfg.server.host, cfg.server.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(bcast.clone()))
            .app_data(web::Data::new(total_logs.clone()))
            .app_data(web::Data::new(registry.clone()))
            .route("/", web::get().to(index))
            .route("/logs", web::post().to(post_log))
            .route("/logs", web::get().to(get_logs))
            .route("/logs/stats", web::get().to(get_stats))
            .route("/logs/stream", web::get().to(logs_stream))
            .route("/metrics", web::get().to(metrics))
    })
    .bind((cfg.server.host, cfg.server.port))?
    .run()
    .await
}
