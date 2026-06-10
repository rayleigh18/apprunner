//! Reverse proxy server for API masks.
//!
//! Listens on a local port, injects configured headers, and forwards
//! requests to the upstream target URL. Streams responses back without buffering.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{watch, Mutex};

use super::log::{latency_since, now_timestamp, LogEntry, RequestLog};

/// Configuration for a mask proxy instance.
#[derive(Debug, Clone)]
pub struct MaskConfig {
    pub name: String,
    pub target_url: String,
    pub listen_port: u16,
    /// Headers to inject (overwrite). Key -> Value.
    pub headers: HashMap<String, String>,
}

/// Handle to a running proxy — used to stop it.
#[derive(Debug)]
pub struct ProxyHandle {
    shutdown_tx: watch::Sender<bool>,
}

impl ProxyHandle {
    /// Signal the proxy to shut down gracefully.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// A running mask proxy instance with its log.
#[derive(Debug)]
pub struct MaskProxy {
    pub config: MaskConfig,
    pub log: Arc<Mutex<RequestLog>>,
    pub handle: ProxyHandle,
}

/// Start a mask proxy server.
///
/// Returns a `MaskProxy` containing the handle (for shutdown) and the shared request log.
/// The proxy runs as a background tokio task.
///
/// Returns an error if the port is already in use.
pub async fn start_proxy(config: MaskConfig) -> Result<MaskProxy> {
    let addr = SocketAddr::from(([127, 0, 0, 1], config.listen_port));

    // Attempt to bind — fail loudly on port conflict
    let listener = TcpListener::bind(addr)
        .await
        .context(format!("port {} already in use", config.listen_port))?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let log = Arc::new(Mutex::new(RequestLog::new()));

    let config_arc = Arc::new(config.clone());
    let log_clone = Arc::clone(&log);

    // Spawn the server loop
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build reqwest client");

        let client = Arc::new(client);

        loop {
            let mut shutdown_rx_inner = shutdown_rx.clone();

            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let io = TokioIo::new(stream);
                            let config = Arc::clone(&config_arc);
                            let client = Arc::clone(&client);
                            let log = Arc::clone(&log_clone);
                            let mut shutdown_rx_conn = shutdown_rx.clone();

                            tokio::spawn(async move {
                                let service = service_fn(move |req| {
                                    let config = Arc::clone(&config);
                                    let client = Arc::clone(&client);
                                    let log = Arc::clone(&log);
                                    async move {
                                        handle_request(req, &config, &client, &log).await
                                    }
                                });

                                let conn = http1::Builder::new()
                                    .serve_connection(io, service);

                                tokio::pin!(conn);

                                tokio::select! {
                                    result = &mut conn => {
                                        if let Err(e) = result {
                                            // Connection errors are normal (client disconnect)
                                            let _ = e;
                                        }
                                    }
                                    _ = shutdown_rx_conn.changed() => {
                                        // Graceful shutdown — let connection drain
                                        conn.as_mut().graceful_shutdown();
                                        let _ = conn.await;
                                    }
                                }
                            });
                        }
                        Err(_) => {
                            // Accept error — continue listening
                            continue;
                        }
                    }
                }
                _ = shutdown_rx_inner.changed() => {
                    // Shutdown signal received
                    break;
                }
            }
        }
    });

    Ok(MaskProxy {
        config,
        log,
        handle: ProxyHandle { shutdown_tx },
    })
}

/// Handle a single proxied request.
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    config: &MaskConfig,
    client: &reqwest::Client,
    log: &Arc<Mutex<RequestLog>>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let start = Instant::now();
    let method_str = req.method().to_string();
    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());

    // Build the upstream URL
    let upstream_url = format!("{}{}", config.target_url.trim_end_matches('/'), &path);

    // Convert the request
    let method = req.method().clone();

    // Collect original headers (we'll overwrite mask headers)
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in req.headers() {
        if key == hyper::header::HOST {
            continue; // Don't forward the localhost Host header
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_str().as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                headers.insert(name, val);
            }
        }
    }

    // Overwrite with mask headers (always overwrite)
    for (key, value) in &config.headers {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(value) {
                headers.insert(name, val);
            }
        }
    }

    // Collect request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    // Build and send the upstream request
    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let upstream_req = client
        .request(reqwest_method, &upstream_url)
        .headers(headers)
        .body(body_bytes.to_vec());

    let response = match upstream_req.send().await {
        Ok(resp) => resp,
        Err(e) => {
            // Log the error (never include header values)
            let error_msg = format!("upstream error: {}", sanitize_error(&e.to_string(), config));
            let latency = latency_since(start);

            let mut log_guard = log.lock().await;
            log_guard.push(LogEntry {
                timestamp: now_timestamp(),
                method: method_str,
                path,
                status_code: None,
                latency_ms: latency,
                error: Some(error_msg),
            });

            // Return 502 Bad Gateway
            let body = Full::new(Bytes::from("Bad Gateway"))
                .map_err(|never| match never {})
                .boxed();
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(body)
                .unwrap());
        }
    };

    // Log successful response at header time (TTFB)
    let status = response.status().as_u16();
    let latency = latency_since(start);

    {
        let mut log_guard = log.lock().await;
        log_guard.push(LogEntry {
            timestamp: now_timestamp(),
            method: method_str,
            path,
            status_code: Some(status),
            latency_ms: latency,
            error: None,
        });
    }

    // Build the response to send back to the caller
    let mut builder = Response::builder().status(response.status());

    // Forward all response headers
    for (key, value) in response.headers() {
        builder = builder.header(key.as_str(), value.as_bytes());
    }

    // Stream the response body back chunk-by-chunk
    let stream = response
        .bytes_stream()
        .map_ok(Frame::data)
        .map_err(|_e| {
            // Stream errors will cause the connection to drop
            unreachable!("stream error")
        });

    let body: BoxBody<Bytes, hyper::Error> = StreamBody::new(stream).boxed();

    Ok(builder.body(body).unwrap())
}

/// Remove any header values from error messages for security.
fn sanitize_error(error: &str, config: &MaskConfig) -> String {
    let mut sanitized = error.to_string();
    for value in config.headers.values() {
        sanitized = sanitized.replace(value, "[REDACTED]");
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer sk-secret-key".to_string());

        let config = MaskConfig {
            name: "test".to_string(),
            target_url: "https://api.example.com".to_string(),
            listen_port: 8080,
            headers,
        };

        let error = "connection failed with Bearer sk-secret-key in header";
        let sanitized = sanitize_error(error, &config);
        assert!(!sanitized.contains("sk-secret-key"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_mask_config_creation() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test".to_string());

        let config = MaskConfig {
            name: "openai".to_string(),
            target_url: "https://api.openai.com".to_string(),
            listen_port: 8080,
            headers,
        };

        assert_eq!(config.name, "openai");
        assert_eq!(config.listen_port, 8080);
    }

    #[tokio::test]
    async fn test_start_proxy_port_conflict() {
        let config1 = MaskConfig {
            name: "first".to_string(),
            target_url: "https://api.example.com".to_string(),
            listen_port: 19876, // Use a high port unlikely to conflict
            headers: HashMap::new(),
        };

        let proxy1 = start_proxy(config1).await.unwrap();

        // Try to bind the same port again
        let config2 = MaskConfig {
            name: "second".to_string(),
            target_url: "https://api.example.com".to_string(),
            listen_port: 19876,
            headers: HashMap::new(),
        };

        let result = start_proxy(config2).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already in use"));

        // Clean up
        proxy1.handle.shutdown();
    }
}
