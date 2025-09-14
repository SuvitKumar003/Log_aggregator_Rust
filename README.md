# Log Aggregator & Real-Time Monitoring System

A **high-performance, Rust-based log aggregation and monitoring system** with real-time dashboard, metrics collection, and asynchronous persistence. Designed to handle logs from multiple microservices efficiently and provide actionable insights instantly.

---

## 🚀 Features

- **Concurrent In-Memory Log Storage:** Handles **50,000+ log entries** efficiently using `Arc<Mutex<Vec<LogEntry>>>`.
- **Real-Time Updates:** Live dashboard updates via **Server-Sent Events (SSE)**.
- **Filterable Logs:** Filter logs by **service** and **level** (INFO, WARN, ERROR).
- **Prometheus Integration:** Tracks **total logs received** and exposes metrics endpoint (`/metrics`) for monitoring system health.
- **Persistent Storage:** Asynchronous saving of logs to disk to ensure durability.
- **Simulated Log Generator:** Generates semi-realistic logs with rare critical events for testing.

---

## 🏗 Architecture

+-------------------+ +--------------------+
| Microservices | ---> | Log Aggregator |
| (Auth, Payments, | | (Rust + Actix) |
| Orders, Shipping) | +--------------------+
+-------------------+ |
v
+-----------------+
| In-Memory DB |
| (50k logs max) |
+-----------------+
|
+-----------------+------------------+
| |
+--------------+ +---------------+
| SSE Stream | | Metrics / |
| (Live Feed) | | Prometheus |
+--------------+ +---------------+


---

## ⚙️ Setup & Installation

1. **Clone the repository**
```bash
git clone <https://github.com/SuvitKumar003/Log_aggregator_Rust>
cd Log_aggregator_Rust
2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

Install dependencies

cargo build


Configure the server

Modify config.toml to set server host, port, logging file path, max logs in memory, and persistence interval.

Run the server

cargo run


Run the log generator (optional)

cargo run --bin log_generator


Access the dashboard

Open your browser at: http://127.0.0.1:8080/

🖥 Dashboard Screenshots
<!-- Replace the paths with your actual screenshots -->


<img width="1877" height="878" alt="Screenshot 2025-09-14 183952" src="https://github.com/user-attachments/assets/bf452147-9a6e-4554-abce-aa8fe8446a87" />

<img width="1911" height="857" alt="Screenshot 2025-09-14 184007" src="https://github.com/user-attachments/assets/54dcc71b-54d3-4202-be1a-5d8bc80861d6" />

<img width="1916" height="848" alt="Screenshot 2025-09-14 184023" src="https://github.com/user-attachments/assets/6585b2a0-1307-4265-9d25-c117ae0f40bf" />
![Uploading Screenshot 2025-09-14 184037.png…]()

🔧 Tech Stack

Backend: Rust, Actix-Web, Tokio

Frontend: HTML, CSS, JavaScript

Metrics: Prometheus, TextEncoder

Data Handling: Arc<Mutex>, broadcast channel

Utilities: Chrono, Serde, Reqwest (for log generation)

💡 Highlights / USP

Handles 50,000+ logs efficiently in-memory with minimal latency.

Real-time, filterable dashboard to instantly view logs by service and severity.

Asynchronous persistence ensures durability without blocking the server.

Prometheus integration enables external monitoring and alerting.

📈 Use Cases

Microservices monitoring in production.

Simulated logging for testing dashboards or alert systems.

Educational tool for Rust, Actix-Web, and real-time SSE applications.

📝 License

MIT License © [Suvit Kumar]
