// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub full_name: String, // "owner/name"
    pub description: Option<String>,
    pub ssh_url: String,
    pub clone_url: String,
    pub archived: bool,
    pub fork: bool,
    pub private: bool,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub updated_at: String, // ISO8601
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    #[allow(dead_code)]
    pub login: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoredAuth {
    pub github_token: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TokenOkResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub scope: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenErrResponse {
    pub error: String,
    pub error_description: Option<String>,
}
