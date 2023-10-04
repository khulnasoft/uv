use std::path::{Path, PathBuf};
use std::sync::Arc;

use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::ClientBuilder;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use url::Url;

mod api;
mod error;

#[derive(Debug, Clone)]
pub struct PypiClientBuilder {
    registry: Url,
    retries: u32,
    cache: Option<PathBuf>,
}

impl Default for PypiClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://pypi.org").unwrap(),
            cache: None,
            retries: 0,
        }
    }
}

impl PypiClientBuilder {
    #[must_use]
    pub fn registry(mut self, registry: Url) -> Self {
        self.registry = registry;
        self
    }

    #[must_use]
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    #[must_use]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn build(self) -> PypiClient {
        let client_raw = {
            let client_core = ClientBuilder::new()
                .user_agent("puffin")
                .pool_max_idle_per_host(20)
                .timeout(std::time::Duration::from_secs(60 * 5));

            client_core.build().expect("Fail to build HTTP client.")
        };

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(self.retries);
        let retry_strategy = RetryTransientMiddleware::new_with_policy(retry_policy);

        let mut client_builder =
            reqwest_middleware::ClientBuilder::new(client_raw).with(retry_strategy);

        if let Some(path) = self.cache {
            client_builder = client_builder.with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager { path },
                options: HttpCacheOptions::default(),
            }));
        }

        PypiClient {
            registry: Arc::new(self.registry),
            client: client_builder.build(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PypiClient {
    pub(crate) registry: Arc<Url>,
    pub(crate) client: ClientWithMiddleware,
}
