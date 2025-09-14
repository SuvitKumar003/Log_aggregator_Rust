use actix_files::NamedFile;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

// ---------- Data Model ----------
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    timestamp: String,
    service: String,
    level: String,
    message: String,
}

// ---------- Storage Trait ----------
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn add_log(&self, log: LogEntry);
    async fn get_logs(
        &self,
        service: Option<String>,
        level: Option<String>,
    ) -> Vec<LogEntry>;
    async fn get_stats(&self) -> serde_json::Value;
}

// ---------- In-Memory Storage ----------
pub struct InMemoryStorage {
    db: Arc<Mutex<Vec<LogEntry>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn add_log(&self, log: LogEntry) {
        let mut db = self.db.lock().unwrap();
        db.push(log);
        if db.len() > 50_000 {
            let len = db.len();
            db.drain(0..(len - 50_000));
        }
    }

    async fn get_logs(
        &self,
        service: Option<String>,
        level: Option<String>,
    ) -> Vec<LogEntry> {
        let db = self.db.lock().unwrap();
        let mut logs = db.clone();
        if let Some(s) = service {
            logs.retain(|l| l.service == s);
        }
        if let Some(lv) = level {
            logs.retain(|l| l.level == lv);
        }
        logs
    }

    async fn get_stats(&self) -> serde_json::Value {
        let db = self.db.lock().unwrap();
        let mut by_level = std::collections::HashMap::new();
        let mut by_service = std::collections::HashMap::new();

        for log in db.iter() {
            *by_level.entry(log.level.clone()).or_insert(0) += 1;
            *by_service.entry(log.service.clone()).or_insert(0) += 1;
        }

        serde_json::json!({
            "by_level": by_level,
            "by_service": by_service
        })
    }
}

// ---------- Shared Types ----------
type SharedStorage = Arc<dyn Storage>;
type Broadcaster = Arc<broadcast::Sender<String>>;

// ---------- Handlers ----------

// POST /logs
async fn post_log(
    storage: web::Data<SharedStorage>,
    bcast: web::Data<Broadcaster>,
    log: web::Json<LogEntry>,
) -> impl Responder {
    let mut entry = log.into_inner();

    if entry.timestamp.trim().is_empty() {
        entry.timestamp = Utc::now().to_rfc3339();
    }

    storage.add_log(entry.clone()).await;

    if let Ok(payload) = serde_json::to_string(&entry) {
        let _ = bcast.send(payload);
    }

    HttpResponse::Ok().body("Log added")
}

// GET /logs?service=...&level=...
async fn get_logs(
    storage: web::Data<SharedStorage>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let service = query.get("service").cloned();
    let level = query.get("level").cloned();
    let logs = storage.get_logs(service, level).await;
    HttpResponse::Ok().json(logs)
}

// GET /logs/stats
async fn get_stats(storage: web::Data<SharedStorage>) -> impl Responder {
    let stats = storage.get_stats().await;
    HttpResponse::Ok().json(stats)
}

// GET / → serve index.html
async fn index() -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("static/index.html")?)
}

// GET /logs/stream → SSE stream
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

// ---------- Main ----------
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let storage: SharedStorage = Arc::new(InMemoryStorage::new());
    let (tx, _) = broadcast::channel(1000);
    let bcast: Broadcaster = Arc::new(tx);

    // 🔹 Spawn concurrent log generators
    for i in 0..5 {
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            loop {
                let log = LogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    service: format!("service_{}", i),
                    level: "INFO".to_string(),
                    message: format!("Hello from task {}", i),
                };
                storage_clone.add_log(log).await;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    }

    println!("Server running at http://127.0.0.1:8080/");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(storage.clone()))
            .app_data(web::Data::new(bcast.clone()))
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
