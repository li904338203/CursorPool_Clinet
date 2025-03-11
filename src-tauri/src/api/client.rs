use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

// 共享的 HTTP 客户端
#[derive(Clone)]
pub struct ApiClient(pub(crate) Arc<Client>);

impl Default for ApiClient {
    fn default() -> Self {
        Self(Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(10))
                // 允许不安全的 HTTP 连接
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Failed to create HTTP client"),
        ))
    }
}

// 从环境变量获取基础 URL
pub fn get_base_url() -> String {
    // 保留旧的 URL 以便兼容性，但新的接口会直接使用完整 URL
    "".to_string()
}
