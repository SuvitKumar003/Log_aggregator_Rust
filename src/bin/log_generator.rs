use rand::Rng;
use reqwest::blocking::Client;
use serde_json::json;
use std::{thread, time::Duration};

fn main() {
    // Create HTTP client
    let client = Client::new();

    let services = ["auth", "payments", "orders", "inventory", "shipping"];
    let levels = ["INFO", "WARN", "ERROR"];

    for i in 1..=50 {
        let mut rng = rand::thread_rng();

        let service = services[rng.gen_range(0..services.len())];
        let level = levels[rng.gen_range(0..levels.len())];
        let message = format!("Random log message {}", rng.gen_range(1..1000));

        // Create JSON payload
        let payload = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "service": service,
            "level": level,
            "message": message
        });

        // POST to the server
        let response = client
            .post("http://127.0.0.1:8080/logs")
            .json(&payload)
            .send();

        match response {
            Ok(resp) => println!("Posted log: {} {} -> {}", service, level, resp.status()),
            Err(err) => println!("Error posting log: {}", err),
        }

        // Wait 200ms between logs
        thread::sleep(Duration::from_millis(200));
    }

    println!("✅ Finished posting logs");
}
