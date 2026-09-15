use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Result, anyhow};

use super::executor::{adb_output_message, ensure_success, run_adb};
use super::types::{EnableProxyPayload, ProxyState};

pub async fn get_http_proxy(adb_path: &Path, serial: &str) -> Result<Option<String>> {
    let output = run_adb(
        adb_path,
        Some(serial),
        &["shell", "settings", "get", "global", "http_proxy"],
    )
    .await?;
    ensure_success(&output, "get http_proxy")?;
    let value = adb_output_message(&output).trim().to_string();
    if value.is_empty() || value == "null" {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub async fn set_http_proxy(adb_path: &Path, serial: &str, value: &str) -> Result<()> {
    let output = run_adb(
        adb_path,
        Some(serial),
        &["shell", "settings", "put", "global", "http_proxy", value],
    )
    .await?;
    ensure_success(&output, "set http_proxy")?;
    Ok(())
}

pub fn pick_lan_host(access_addrs: &[SocketAddr], override_host: Option<&str>) -> Result<String> {
    if let Some(host) = override_host {
        let host = host.trim();
        if host.is_empty() {
            return Err(anyhow!("host must not be empty"));
        }
        return Ok(strip_port_from_host(host));
    }
    for addr in access_addrs {
        let ip = addr.ip();
        if ip.is_loopback() {
            continue;
        }
        return Ok(ip.to_string());
    }
    Err(anyhow!(
        "no LAN address available; restart Lynx without --local-only"
    ))
}

fn strip_port_from_host(host: &str) -> String {
    if host.starts_with('[')
        && let Some(end) = host.find(']')
    {
        return host[1..end].to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

pub fn proxy_host_port(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

fn is_proxy_enabled(current_http_proxy: Option<&str>) -> bool {
    matches!(current_http_proxy, Some(v) if v.trim() != ":0")
}

pub async fn get_proxy_state(
    adb_path: &Path,
    _data_root: &Path,
    serial: &str,
) -> Result<ProxyState> {
    let current = get_http_proxy(adb_path, serial).await?;
    Ok(ProxyState {
        serial: serial.to_string(),
        lynx_managed: is_proxy_enabled(current.as_deref()),
        current_http_proxy: current,
    })
}

pub async fn enable_proxy(
    adb_path: &Path,
    data_root: &Path,
    access_addrs: &[SocketAddr],
    local_only: bool,
    payload: EnableProxyPayload,
) -> Result<ProxyState> {
    if local_only {
        return Err(anyhow!(
            "proxy enable is unavailable while Lynx runs with --local-only"
        ));
    }

    let port = payload
        .port
        .unwrap_or_else(|| access_addrs.first().map(|a| a.port()).unwrap_or(7788));

    let host = pick_lan_host(access_addrs, Some(payload.host.as_str()))?;
    let proxy_value = proxy_host_port(&host, port);
    set_http_proxy(adb_path, &payload.serial, &proxy_value).await?;

    get_proxy_state(adb_path, data_root, &payload.serial).await
}

pub async fn disable_proxy(adb_path: &Path, data_root: &Path, serial: &str) -> Result<ProxyState> {
    set_http_proxy(adb_path, serial, ":0").await?;
    get_proxy_state(adb_path, data_root, serial).await
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;

    #[test]
    fn pick_lan_host_from_access_list() {
        let addrs = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7788),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)), 7788),
        ];
        let host = pick_lan_host(&addrs, None).unwrap();
        assert_eq!(host, "192.0.2.10");
    }

    #[test]
    fn pick_lan_host_override() {
        let host = pick_lan_host(&[], Some("10.0.0.2:9999")).unwrap();
        assert_eq!(host, "10.0.0.2");
    }

    #[test]
    fn pick_lan_host_fails_loopback_only() {
        let addrs = vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            7788,
        )];
        assert!(pick_lan_host(&addrs, None).is_err());
    }
}
