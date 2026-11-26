use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use url::Url;

use crate::oauth::client::{CheckedClient, ClientMetadata, check_metadata};

pub const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_DOCUMENT_BYTES: usize = 5 * 1024;

pub async fn fetch_client_metadata(client_id: &str) -> Option<CheckedClient> {
    tokio::time::timeout(FETCH_TIMEOUT, fetch_within_limit(client_id))
        .await
        .ok()?
}

async fn fetch_within_limit(client_id: &str) -> Option<CheckedClient> {
    let url = document_url(client_id)?;
    let host = url.host_str()?.to_string();
    let address = public_address(&host, url.port_or_known_default()?).await?;
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve(&host, address)
        .build()
        .ok()?;
    let bytes = download(&client, url.as_str(), MAX_DOCUMENT_BYTES).await?;
    parse_document(client_id, &bytes)
}

fn document_url(client_id: &str) -> Option<Url> {
    let url = Url::parse(client_id).ok()?;
    let usable = url.scheme() == "https"
        && url.fragment().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() != "/"
        && matches!(url.host(), Some(url::Host::Domain(_)));
    usable.then_some(url)
}

pub(super) async fn download(client: &reqwest::Client, url: &str, limit: usize) -> Option<Vec<u8>> {
    let mut response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        body.extend_from_slice(&chunk);
        if body.len() > limit {
            return None;
        }
    }
    Some(body)
}

fn parse_document(client_id: &str, bytes: &[u8]) -> Option<CheckedClient> {
    let metadata: ClientMetadata = serde_json::from_slice(bytes).ok()?;
    if metadata.client_id.as_deref() != Some(client_id) {
        return None;
    }
    check_metadata(&metadata).ok()
}

async fn public_address(host: &str, port: u16) -> Option<SocketAddr> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port)).await.ok()?.collect();
    if addresses.is_empty() || !addresses.iter().all(|address| is_public(address.ip())) {
        return None;
    }
    addresses.first().copied()
}

pub(super) fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_v4(ip),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(mapped) => is_public_v4(mapped),
            None => is_public_v6(ip),
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [first, second, ..] = ip.octets();
    let shared = first == 100 && (64..128).contains(&second);
    let reserved = first == 0 || first >= 240;
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || shared
        || reserved)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    let unique_local = first & 0xfe00 == 0xfc00;
    let link_local = first & 0xffc0 == 0xfe80;
    let documentation = first == 0x2001 && ip.segments()[1] == 0x0db8;
    let nat64 = ip.segments()[..6] == [0x64, 0xff9b, 0, 0, 0, 0];
    let six_to_four = first == 0x2002;
    let ipv4_compatible = ip.segments()[..6] == [0; 6];
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || unique_local
        || link_local
        || documentation
        || nat64
        || six_to_four
        || ipv4_compatible)
}

#[cfg(test)]
mod tests;
