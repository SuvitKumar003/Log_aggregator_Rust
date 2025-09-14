use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use futures::StreamExt;
use bytes::Bytes;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LogEntry {
    timestamp: String,
    service: String,
    level: String,
    message: String,
}

// Shared in-memory storage
type LogDb = Arc<Mutex<Vec<LogEntry>>>;
type Broadcaster = Arc<broadcast::Sender<String>>;

// POST /logs
async fn post_log(
    db: web::Data<LogDb>,
    bcast: web::Data<Broadcaster>,
    log: web::Json<LogEntry>,
) -> impl Responder {
    let mut entry = log.into_inner();

    // Add timestamp if missing
    if entry.timestamp.trim().is_empty() {
        entry.timestamp = Utc::now().to_rfc3339();
    }

    {
        let mut db_lock = db.lock().unwrap();
        db_lock.push(entry.clone());

        // Optional: keep memory bounded
        let len = db_lock.len();
        if len > 50_000 {
            db_lock.drain(0..(len - 50_000));
        }
    }

    // Broadcast to SSE subscribers
    if let Ok(payload) = serde_json::to_string(&entry) {
        let _ = bcast.send(payload);
    }

    HttpResponse::Ok().body("Log added")
}

// GET /logs?service=...&level=...
async fn get_logs(db: web::Data<LogDb>, query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
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

// NEW: SSE stream at /logs/stream
async fn logs_stream(bcast: web::Data<Broadcaster>) -> HttpResponse {
    let rx = bcast.subscribe();

    let stream = futures::stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(msg) => {
                let sse_frame = format!("data: {}\n\n", msg);
                Some((Ok::<Bytes, actix_web::Error>(Bytes::from(sse_frame)), rx))
            }
            Err(_) => None,
        }
    });

    HttpResponse::Ok()
        .append_header(("Content-Type", "text/event-stream"))
        .append_header(("Cache-Control", "no-cache"))
        .streaming(stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db: LogDb = Arc::new(Mutex::new(Vec::new()));

    // Create broadcaster for SSE
    let (tx, _rx) = broadcast::channel::<String>(100);
    let broadcaster: Broadcaster = Arc::new(tx);

    println!("Server running at http://127.0.0.1:8080/");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(broadcaster.clone()))
            .route("/", web::get().to(index))
            .route("/logs", web::post().to(post_log))
            .route("/logs", web::get().to(get_logs))
            .route("/logs/stats", web::get().to(get_stats))
            .route("/logs/stream", web::get().to(logs_stream))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
