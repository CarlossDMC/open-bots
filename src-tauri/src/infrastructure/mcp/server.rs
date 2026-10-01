use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    body::{Bytes, Incoming},
    header::{self, HeaderValue},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use serde_json::Value;

use super::protocol::{handle_message, parse_error};
use crate::{
    error::{AppError, AppResult},
    runtime::turn_tokens::TurnTokens,
    tools::ToolHost,
};

/// Largest request body accepted; tool inputs are small JSON objects.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const ENDPOINT_PATH: &str = "/mcp";

/// A loopback listener bound before the async runtime serves it, so the URL is known while
/// the rest of the application is wired.
pub struct McpListener {
    listener: std::net::TcpListener,
    address: SocketAddr,
}

impl McpListener {
    /// Binds `127.0.0.1` on a port chosen by the operating system.
    pub fn bind() -> AppResult<Self> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| AppError::Process(format!("MCP server could not bind: {error}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| AppError::Process(format!("MCP server setup failed: {error}")))?;
        let address = listener
            .local_addr()
            .map_err(|error| AppError::Process(format!("MCP server setup failed: {error}")))?;
        Ok(Self { listener, address })
    }

    pub fn url(&self) -> String {
        format!("http://{}{ENDPOINT_PATH}", self.address)
    }

    /// Serves connections until the process exits. Must run inside a Tokio runtime.
    pub async fn serve(self, host: Arc<dyn ToolHost>, tokens: Arc<TurnTokens>) {
        let listener = match tokio::net::TcpListener::from_std(self.listener) {
            Ok(listener) => listener,
            Err(error) => {
                tracing::error!(%error, "MCP server could not start");
                return;
            }
        };
        tracing::info!(address = %self.address, "MCP server listening");
        loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(error) => {
                    tracing::warn!(%error, "MCP connection was not accepted");
                    continue;
                }
            };
            let host = Arc::clone(&host);
            let tokens = Arc::clone(&tokens);
            tokio::spawn(async move {
                let service = service_fn(move |request| {
                    let host = Arc::clone(&host);
                    let tokens = Arc::clone(&tokens);
                    async move { Ok::<_, Infallible>(respond(request, host, tokens).await) }
                });
                if let Err(error) = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await
                {
                    tracing::debug!(%error, "MCP connection closed with an error");
                }
            });
        }
    }
}

async fn respond(
    request: Request<Incoming>,
    host: Arc<dyn ToolHost>,
    tokens: Arc<TurnTokens>,
) -> Response<Full<Bytes>> {
    if request.uri().path() != ENDPOINT_PATH {
        return status(StatusCode::NOT_FOUND);
    }
    // Browsers send Origin; a page on another site must not reach the local server.
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        if !is_loopback_origin(origin) {
            return status(StatusCode::FORBIDDEN);
        }
    }
    if request.method() != Method::POST {
        let mut response = status(StatusCode::METHOD_NOT_ALLOWED);
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("POST"));
        return response;
    }
    let context = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| tokens.resolve(token.trim()));
    let Some(context) = context else {
        let mut response = status(StatusCode::UNAUTHORIZED);
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        return response;
    };
    let body = match Limited::new(request.into_body(), MAX_REQUEST_BYTES)
        .collect()
        .await
    {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return status(StatusCode::PAYLOAD_TOO_LARGE),
    };
    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        return json(StatusCode::BAD_REQUEST, &parse_error());
    };
    if let Some(method) = message.get("method").and_then(Value::as_str) {
        tracing::debug!(agent_id = %context.agent_id, method, "MCP request");
    }
    match handle_message(host.as_ref(), context, &message).await {
        Some(reply) => json(StatusCode::OK, &reply),
        None => status(StatusCode::ACCEPTED),
    }
}

fn is_loopback_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|allowed| {
            origin == *allowed
                || origin
                    .strip_prefix(allowed)
                    .is_some_and(|rest| rest.starts_with(':'))
        })
}

fn status(code: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = code;
    response
}

fn json(code: StatusCode, value: &Value) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from(value.to_string())));
    *response.status_mut() = code;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_loopback_origins() {
        let check =
            |value: &str| is_loopback_origin(&HeaderValue::from_str(value).expect("header"));
        assert!(check("http://127.0.0.1"));
        assert!(check("http://localhost:1420"));
        assert!(!check("http://localhost.example.com"));
        assert!(!check("https://example.com"));
    }
}
