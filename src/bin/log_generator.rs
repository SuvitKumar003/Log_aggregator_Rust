use rand::Rng;
use reqwest::Client;
use serde_json::json;
use chrono::Utc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let client = Client::new();
    let services = vec!["auth", "payments", "orders", "inventory", "shipping"];
    let levels = vec!["INFO", "WARN", "ERROR"];

    loop {
        let service = services[rand::thread_rng().gen_range(0..services.len())];
        let level = levels[rand::thread_rng().gen_range(0..levels.len())];

        let message = generate_message(service, level);

        let log_entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "service": service,
            "level": level,
            "message": message
        });

        let res = client.post("http://127.0.0.1:8080/logs")
            .json(&log_entry)
            .send()
            .await;

        match res {
            Ok(_) => println!("Log sent: {:?}", log_entry),
            Err(err) => eprintln!("Error sending log: {:?}", err),
        }

        // Random delay between 1–2 seconds
        sleep(Duration::from_millis(rand::thread_rng().gen_range(1000..2000))).await;
    }
}

// Function to generate semi-realistic log messages with rare critical events
fn generate_message(service: &str, level: &str) -> String {
    let mut rng = rand::thread_rng();

    // 5% chance to trigger a critical event
    if rng.gen_range(0..100) < 5 {
        return format!("CRITICAL ALERT in {} service! Immediate attention required!", service);
    }

    match (service, level) {
        ("auth", "INFO") => "User logged in successfully.".to_string(),
        ("auth", "WARN") => "Multiple failed login attempts.".to_string(),
        ("auth", "ERROR") => "Authentication service unavailable!".to_string(),

        ("payments", "INFO") => "Payment processed successfully.".to_string(),
        ("payments", "WARN") => "Payment delayed due to network.".to_string(),
        ("payments", "ERROR") => "Payment transaction failed!".to_string(),

        ("orders", "INFO") => "Order placed successfully.".to_string(),
        ("orders", "WARN") => "Order processing delayed.".to_string(),
        ("orders", "ERROR") => "Failed to place order.".to_string(),

        ("inventory", "INFO") => "Inventory updated.".to_string(),
        ("inventory", "WARN") => "Inventory running low.".to_string(),
        ("inventory", "ERROR") => "Inventory system down!".to_string(),

        ("shipping", "INFO") => "Package shipped.".to_string(),
        ("shipping", "WARN") => "Shipping delayed due to weather.".to_string(),
        ("shipping", "ERROR") => "Shipping service failed!".to_string(),

        _ => "Unknown log message.".to_string(),
    }
}
