use std::cell::RefCell;
use std::rc::Rc;

use async_trait::async_trait;
use luneth::common::ImageData;
use luneth::crawl::{CrawlConfig, CrawlError, CrawlInput, WebCrawler};
use luneth::record::{RecordPiece, Recorder};

use crate::domains::crawl::domain::service::CrawlerTrait;

/// Production crawler wrapping luneth's `WebCrawler`.
/// Uses Rc/RefCell since it runs exclusively on the single crawl-runner thread.
pub struct LunethCrawler {
    inner: Rc<RefCell<Option<WebCrawler>>>,
    started: Rc<RefCell<bool>>,
    config: Rc<RefCell<CrawlConfig>>,
}

impl LunethCrawler {
    pub fn new(crawler: WebCrawler) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Some(crawler))),
            started: Rc::new(RefCell::new(false)),
            config: Rc::new(RefCell::new(CrawlConfig::default())),
        }
    }

    pub fn new_noop() -> Self {
        Self {
            inner: Rc::new(RefCell::new(None)),
            started: Rc::new(RefCell::new(false)),
            config: Rc::new(RefCell::new(CrawlConfig::default())),
        }
    }

    pub async fn ensure_started(&self) -> Result<(), CrawlError> {
        if *self.started.borrow() {
            return Ok(());
        }
        let config_for_retry = self.config.borrow().clone();
        let crawler = self.inner.borrow_mut().take();
        if let Some(crawler) = crawler {
            match crawler.start().await {
                Ok(started) => {
                    *self.inner.borrow_mut() = Some(started);
                    *self.started.borrow_mut() = true;
                }
                Err(e) => {
                    match WebCrawler::with_config(config_for_retry) {
                        Ok(retryable) => {
                            *self.inner.borrow_mut() = Some(retryable);
                        }
                        Err(rebuild_err) => {
                            tracing::error!(
                                "Crawler start failed and retry state rebuild failed: \
                                 start_error={e}; rebuild_error={rebuild_err}"
                            );
                        }
                    }
                    tracing::error!(
                        "Crawler start failed, crawler instance restored for retry: {e}"
                    );
                    return Err(e);
                }
            }
        } else {
            return Err(CrawlError::RecordError(
                "Crawler not initialized".to_owned(),
            ));
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl CrawlerTrait for LunethCrawler {
    async fn set_base_url(&self, base_url: String) -> Result<(), CrawlError> {
        self.config.borrow_mut().base_url = base_url.clone();
        let mut inner = self.inner.borrow_mut();
        if let Some(ref mut crawler) = *inner {
            crawler.set_base_url(base_url);
        } else {
            drop(inner);
            let new_crawler = WebCrawler::with_config(self.config.borrow().clone())?;
            *self.inner.borrow_mut() = Some(new_crawler);
        }
        Ok(())
    }

    #[expect(clippy::await_holding_refcell_ref)]
    async fn crawl_page(&self, url: &str) -> Result<Vec<RecordPiece>, CrawlError> {
        self.ensure_started().await?;
        let mut guard = self.inner.borrow_mut();
        let crawler = guard
            .as_mut()
            .ok_or_else(|| CrawlError::RecordError("Crawler not initialized".to_owned()))?;
        crawler.crawl_page(url).await
    }

    #[expect(clippy::await_holding_refcell_ref)]
    async fn crawl_recorder_with_imgs(
        &self,
        input: CrawlInput,
    ) -> Result<(Recorder, std::sync::Arc<Vec<ImageData>>), CrawlError> {
        self.ensure_started().await?;
        let mut guard = self.inner.borrow_mut();
        let crawler = guard
            .as_mut()
            .ok_or_else(|| CrawlError::RecordError("Crawler not initialized".to_owned()))?;
        crawler.crawl_recorder_with_imgs(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct BridgePathBlocker {
        bridge_path: PathBuf,
        backup_path: Option<PathBuf>,
    }

    impl BridgePathBlocker {
        fn install() -> Self {
            let bridge_path = std::env::temp_dir().join("luneth").join("bridge");
            let backup_path = if bridge_path.exists() {
                let suffix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos();
                let backup = bridge_path.with_file_name(format!("bridge-backup-{suffix}"));
                fs::rename(&bridge_path, &backup).expect("backup bridge path");
                Some(backup)
            } else {
                None
            };

            fs::create_dir_all(bridge_path.parent().expect("bridge parent"))
                .expect("create bridge parent");
            fs::write(&bridge_path, b"blocked").expect("block bridge path");

            Self {
                bridge_path,
                backup_path,
            }
        }
    }

    impl Drop for BridgePathBlocker {
        fn drop(&mut self) {
            drop(fs::remove_file(&self.bridge_path));
            drop(fs::remove_dir_all(&self.bridge_path));
            if let Some(backup) = &self.backup_path {
                drop(fs::rename(backup, &self.bridge_path));
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ensure_started_keeps_crawler_available_after_start_failure() {
        let _blocker = {
            let _guard = env_lock().lock().expect("lock env");
            BridgePathBlocker::install()
        };

        let crawler = LunethCrawler::new(WebCrawler::new().expect("create crawler"));

        let first_err = crawler
            .ensure_started()
            .await
            .expect_err("start should fail when bridge path is blocked");
        assert!(
            first_err.to_string().contains("bridge cache"),
            "unexpected first error: {first_err}"
        );
        assert!(
            crawler.inner.borrow().is_some(),
            "crawler instance should remain available for retry"
        );

        let second_err = crawler
            .ensure_started()
            .await
            .expect_err("retry should attempt startup again");
        assert!(
            second_err.to_string().contains("bridge cache"),
            "unexpected retry error: {second_err}"
        );
    }
}
