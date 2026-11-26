use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

async fn serve_once(body: Option<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 1024];
        let _ = socket.read(&mut request).await;
        let Some(body) = body else {
            tokio::time::sleep(Duration::from_secs(30)).await;
            return;
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{body}"
        );
        let _ = socket.write_all(response.as_bytes()).await;
    });
    format!("http://{address}/client.json")
}

fn quick_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(300))
        .build()
        .unwrap()
}

fn document(client_id: &str) -> String {
    serde_json::json!({
        "client_id": client_id,
        "client_name": "Claude",
        "redirect_uris": ["https://claude.ai/api/mcp/auth_callback"],
        "token_endpoint_auth_method": "none",
    })
    .to_string()
}

#[test]
fn the_fetch_limits_match_the_spec() {
    assert_eq!(FETCH_TIMEOUT, Duration::from_secs(5));
    assert_eq!(MAX_DOCUMENT_BYTES, 5 * 1024);
}

#[tokio::test]
async fn documents_over_the_size_limit_are_refused() {
    let small = serve_once(Some("x".repeat(1024))).await;
    let large = serve_once(Some("x".repeat(MAX_DOCUMENT_BYTES + 1))).await;

    let client = quick_client();
    assert_eq!(
        download(&client, &small, MAX_DOCUMENT_BYTES)
            .await
            .map(|b| b.len()),
        Some(1024)
    );
    assert_eq!(download(&client, &large, MAX_DOCUMENT_BYTES).await, None);
}

#[tokio::test]
async fn a_server_that_never_answers_times_out() {
    let silent = serve_once(None).await;

    assert_eq!(
        download(&quick_client(), &silent, MAX_DOCUMENT_BYTES).await,
        None
    );
}

#[test]
fn the_document_must_name_its_own_url() {
    let url = "https://claude.ai/oauth/client.json";

    let parsed = parse_document(url, document(url).as_bytes()).unwrap();
    assert_eq!(parsed.name, "Claude");
    assert!(parse_document(url, document("https://evil.test/c.json").as_bytes()).is_none());
    assert!(parse_document(url, b"not json").is_none());
}

#[test]
fn only_https_urls_with_a_domain_and_a_path_are_fetched() {
    assert!(document_url("https://claude.ai/oauth/client.json").is_some());
    for refused in [
        "http://claude.ai/client.json",
        "https://claude.ai/",
        "https://93.184.216.34/client.json",
        "https://claude.ai/client.json#x",
        "https://user@claude.ai/client.json",
    ] {
        assert!(document_url(refused).is_none(), "{refused}");
    }
}

#[tokio::test]
async fn hosts_that_resolve_to_private_addresses_are_never_fetched() {
    assert_eq!(public_address("localhost", 443).await, None);
    assert!(
        fetch_client_metadata("https://localhost/client.json")
            .await
            .is_none()
    );
}

#[test]
fn private_and_reserved_addresses_are_not_public() {
    for private in [
        "10.0.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.1.1",
        "100.64.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "::1",
        "fc00::1",
        "fe80::1",
        "::ffff:127.0.0.1",
    ] {
        assert!(!is_public(private.parse().unwrap()), "{private}");
    }
    for public in ["93.184.216.34", "2606:4700::1111"] {
        assert!(is_public(public.parse().unwrap()), "{public}");
    }
}
