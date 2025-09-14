use clap::Parser;
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;

/// Simple CLI to query the log aggregator server
#[derive(Parser, Debug)]
#[command(name = "logcli")]
#[command(about = "Query log aggregator", long_about = None)]
struct Args {
    /// server base URL, e.g., http://127.0.0.1:8080
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    server: String,

    /// filter by service name
    #[arg(short, long)]
    service: Option<String>,

    /// filter by level (INFO/WARN/ERROR/DEBUG)
    #[arg(short, long)]
    level: Option<String>,

    /// show stats instead of logs
    #[arg(long)]
    stats: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let client = Client::new();

    if args.stats {
        let url = format!("{}/logs/stats", args.server.trim_end_matches('/'));
        let resp = client.get(&url).send()?;
        let json: Value = resp.json()?;
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    // Build /logs?params
    let mut url = format!("{}/logs", args.server.trim_end_matches('/'));
    let mut q = vec![];
    if let Some(s) = args.service {
        q.push(format!("service={}", urlencoding::encode(&s)));
    }
    if let Some(lv) = args.level {
        q.push(format!("level={}", urlencoding::encode(&lv)));
    }
    if !q.is_empty() {
        url.push('?');
        url.push_str(&q.join("&"));
    }

    let resp = client.get(&url).send()?;
    let json: Value = resp.json()?;

    // pretty print each log in a readable format
    if let Some(arr) = json.as_array() {
        for entry in arr {
            // Expecting fields: timestamp, service, level, message
            let ts = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("-");
            let svc = entry.get("service").and_then(|v| v.as_str()).unwrap_or("-");
            let level = entry.get("level").and_then(|v| v.as_str()).unwrap_or("-");
            let msg = entry.get("message").and_then(|v| v.as_str()).unwrap_or("-");

            println!("[{}] {} / {} -> {}", ts, svc, level, msg);
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&json)?);
    }

    Ok(())
}
