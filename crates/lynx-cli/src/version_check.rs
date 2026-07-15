use anyhow::Result;
use console::style;
use semver::Version;
use serde::Deserialize;

const GITHUB_API_URL: &str = "https://api.github.com/repos/xin2017338/lynx-proxy/releases/latest";
const RELEASES_URL: &str = "https://github.com/xin2017338/lynx-proxy/releases/latest";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    #[serde(rename = "tag_name")]
    tag_name: String,
}

/// Check if a newer version is available.
/// Returns `Some(latest_version_string)` if an update exists, `None` otherwise.
/// Errors (network, rate-limit, etc.) are silently ignored.
pub async fn check_for_updates() -> Option<String> {
    match check_for_updates_inner().await {
        Ok(Some(version)) => Some(version),
        _ => None,
    }
}

async fn check_for_updates_inner() -> Result<Option<String>> {
    let current = Version::parse(CARGO_PKG_VERSION)?;

    let client = reqwest::Client::builder()
        .user_agent("lynx-proxy")
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    let release: GitHubRelease = client.get(GITHUB_API_URL).send().await?.json().await?;

    let latest_tag = release.tag_name.trim_start_matches('v');
    let latest = Version::parse(latest_tag)?;

    if latest > current {
        Ok(Some(latest_tag.to_string()))
    } else {
        Ok(None)
    }
}

/// Print a non-interactive update reminder.
pub fn print_update_banner(latest: &str) {
    println!();
    println!(
        "{}{}",
        style("Update available: ").bold().green(),
        style(format!("v{} (current: v{})", latest, CARGO_PKG_VERSION)).cyan(),
    );
    println!("  {}", style(RELEASES_URL).underlined().cyan(),);
}
