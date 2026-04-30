use std::cell::RefCell;
use std::rc::Rc;

use async_trait::async_trait;
use luneth::common::ImageData;
use luneth::crawl::{CrawlError, CrawlInput, WebCrawler};
use luneth::record::{RecordPiece, Recorder};

use crate::domains::crawl::domain::service::CrawlerTrait;

/// Production crawler wrapping luneth's `WebCrawler`.
/// Uses Rc/RefCell since it runs exclusively on the single crawl-runner thread.
pub struct LunethCrawler {
    inner: Rc<RefCell<Option<WebCrawler>>>,
    started: Rc<RefCell<bool>>,
}

impl LunethCrawler {
    pub fn new(crawler: WebCrawler) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Some(crawler))),
            started: Rc::new(RefCell::new(false)),
        }
    }

    pub fn new_noop() -> Self {
        Self {
            inner: Rc::new(RefCell::new(None)),
            started: Rc::new(RefCell::new(false)),
        }
    }

    async fn ensure_started(&self) -> Result<(), CrawlError> {
        if *self.started.borrow() {
            return Ok(());
        }
        let crawler = self.inner.borrow_mut().take();
        if let Some(crawler) = crawler {
            match crawler.start().await {
                Ok(started) => {
                    *self.inner.borrow_mut() = Some(started);
                    *self.started.borrow_mut() = true;
                }
                Err(e) => {
                    tracing::error!("Crawler start failed, crawler instance lost: {e}");
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
    #[expect(clippy::await_holding_refcell_ref)]
    async fn crawl_page(&self, url: &str) -> Result<Vec<RecordPiece>, CrawlError> {
        self.ensure_started().await?;
        let guard = self.inner.borrow();
        let crawler = guard
            .as_ref()
            .ok_or_else(|| CrawlError::RecordError("Crawler not initialized".to_owned()))?;
        crawler.crawl_page(url).await
    }

    #[expect(clippy::await_holding_refcell_ref)]
    async fn crawl_recorder_with_imgs(
        &self,
        input: CrawlInput,
    ) -> Result<(Recorder, std::sync::Arc<Vec<ImageData>>), CrawlError> {
        self.ensure_started().await?;
        let guard = self.inner.borrow();
        let crawler = guard
            .as_ref()
            .ok_or_else(|| CrawlError::RecordError("Crawler not initialized".to_owned()))?;
        crawler.crawl_recorder_with_imgs(input).await
    }
}
