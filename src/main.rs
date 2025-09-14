use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files::NamedFile;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use futures::stream::Stream;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LogEntry {
    timestamp: String,
    service: String,
    level: String,
    message: String,
}

// Shared in-memory storage
type LogDb = Arc<Mutex<Vec<LogEntry>>>;

// POST /logs
async fn post_log(db: web::Data<LogDb>, log: web::Json<LogEntry>) -> impl Responder {
    let mut db_lock = db.lock().unwrap();
    db_lock.push(log.into_inner());
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db: LogDb = Arc::new(Mutex::new(Vec::new()));
    println!("Server running at http://127.0.0.1:8080/");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .route("/", web::get().to(index))
            .route("/logs", web::post().to(post_log))
            .route("/logs", web::get().to(get_logs))
            .route("/logs/stats", web::get().to(get_stats))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
