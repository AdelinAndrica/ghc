// src/github.rs
use anyhow::{Context, Result, anyhow};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use tokio::time::{Duration, sleep};

use crate::models::{GitHubUser, Repo};

fn github_client(token: &str) -> Result<(reqwest::Client, HeaderMap)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to initialize HTTP client")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("ghc-cli"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );

    Ok((client, headers))
}

pub async fn validate_token(token: &str) -> Result<()> {
    let (client, headers) = github_client(token)?;

    let resp = client
        .get("https://api.github.com/user")
        .headers(headers.clone())
        .send()
        .await
        .with_context(
            || "Failed to contact https://api.github.com/user (network/DNS/proxy/firewall?)",
        )?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!(
            "Authentication expired/invalid. Run `ghc login` again."
        ));
    }

    resp.error_for_status()
        .context("GitHub API returned an error for /user")?
        .json::<GitHubUser>()
        .await
        .context("Failed to parse GitHub /user response")?;

    Ok(())
}

pub async fn fetch_repos(token: &str, owned_only: bool) -> Result<Vec<Repo>> {
    let (client, headers) = github_client(token)?;

    let mut page = 1u32;
    let mut all: Vec<Repo> = Vec::new();

    loop {
        let mut url = format!("https://api.github.com/user/repos?per_page=100&page={page}");
        if owned_only {
            url.push_str("&affiliation=owner");
        } else {
            url.push_str("&affiliation=owner,collaborator,organization_member");
        }

        let mut attempt = 0u32;
        let batch: Vec<Repo> = loop {
            attempt += 1;

            let resp = client.get(&url).headers(headers.clone()).send().await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    let transient = e.is_connect() || e.is_timeout() || e.is_request();
                    if transient && attempt < 3 {
                        sleep(Duration::from_millis(600 * attempt as u64)).await;
                        continue;
                    }
                    return Err(anyhow!(
                        "Failed to contact GitHub API (DNS/proxy/firewall?). Details: {e}"
                    ));
                }
            };

            if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                return Err(anyhow!(
                    "Authentication expired/invalid. Run `ghc login` again."
                ));
            }

            let resp = resp
                .error_for_status()
                .context("GitHub API returned an error")?;

            let parsed = resp
                .json::<Vec<Repo>>()
                .await
                .context("Failed to parse GitHub API response")?;

            break parsed;
        };

        if batch.is_empty() {
            break;
        }

        all.extend(batch);
        page += 1;
    }

    Ok(all)
}
