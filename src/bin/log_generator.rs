use reqwest::Client;
use serde::Serialize;
use chrono::Utc;
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize)]
struct LogEntry {
    timestamp: String,
    service: String,
    level: String,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let services = vec!["auth", "payment", "inventory", "analytics"];
    let levels = vec!["INFO", "WARN", "ERROR", "DEBUG"];

    for i in 0..10 {
        let service = services[rand::thread_rng().gen_range(0..services.len())].to_string();
        let level = levels[rand::thread_rng().gen_range(0..levels.len())].to_string();

        let log = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            service: service.clone(),
            level: level.clone(),
            message: format!("Log message {} from {}", i, service),
        };

        let res = client
            .post("http://127.0.0.1:8080/logs")
            .json(&log)
            .send()
            .await?;

        println!("Sent log {} [{} - {}], response: {}", i, service, level, res.status());

        // ✅ NEW: slow down so logs appear one by one
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
