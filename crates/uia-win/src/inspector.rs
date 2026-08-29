use crate::mock::MockUiaStore;
use crate::model::UiaElementInfo;
use core_types::metadata::TargetMetadata;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

pub enum UiaRequest {
    Point(i32, i32, tokio::sync::oneshot::Sender<Option<UiaElementInfo>>),
    Focused(tokio::sync::oneshot::Sender<Option<UiaElementInfo>>),
}

/// Thread-safe UIA inspector with 100ms timeout fallback.
#[derive(Clone)]
pub struct UiaInspector {
    tx: Option<tokio::sync::mpsc::Sender<UiaRequest>>,
    mock_store: Option<Arc<MockUiaStore>>,
}

impl UiaInspector {
    /// Initialize with native COM STA worker or fallback to mock store.
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<UiaRequest>(100);

            // Spawn dedicated STA COM thread
            std::thread::Builder::new()
                .name("uia-sta-worker".to_string())
                .spawn(move || {
                    let ctx_res = crate::walker::native::NativeUiaContext::init();
                    let ctx = match ctx_res {
                        Ok(c) => Some(c),
                        Err(e) => {
                            warn!("Failed to initialize NativeUiaContext: {:?}", e);
                            None
                        }
                    };

                    while let Some(req) = rx.blocking_recv() {
                        match req {
                            UiaRequest::Point(x, y, resp) => {
                                let info = ctx.as_ref().and_then(|c| c.element_from_point(x, y));
                                let _ = resp.send(info);
                            }
                            UiaRequest::Focused(resp) => {
                                let info = ctx.as_ref().and_then(|c| c.get_focused_element());
                                let _ = resp.send(info);
                            }
                        }
                    }
                })
                .expect("Failed to spawn UIA STA worker thread");

            Self {
                tx: Some(tx),
                mock_store: None,
            }
        }

        #[cfg(not(windows))]
        {
            Self::new_mock()
        }
    }

    /// Initialize with an in-memory mock store.
    pub fn new_mock() -> Self {
        Self {
            tx: None,
            mock_store: Some(Arc::new(MockUiaStore::new())),
        }
    }

    pub fn with_mock_store(store: Arc<MockUiaStore>) -> Self {
        Self {
            tx: None,
            mock_store: Some(store),
        }
    }

    pub fn mock_store(&self) -> Option<&Arc<MockUiaStore>> {
        self.mock_store.as_ref()
    }

    /// Query element at screen coordinate with a 100ms timeout.
    pub async fn inspect_point(&self, x: i32, y: i32) -> Option<TargetMetadata> {
        if let Some(ref mock) = self.mock_store {
            return mock.find_at_point(x, y).map(|e| e.to_target_metadata());
        }

        if let Some(ref tx) = self.tx {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            if tx.send(UiaRequest::Point(x, y, resp_tx)).await.is_err() {
                return None;
            }

            match tokio::time::timeout(Duration::from_millis(100), resp_rx).await {
                Ok(Ok(Some(info))) => Some(info.to_target_metadata()),
                Ok(Ok(None)) => None,
                Ok(Err(_)) => None,
                Err(_) => {
                    debug!("UIA query at ({}, {}) timed out (>100ms), falling back", x, y);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Query currently focused element with a 100ms timeout.
    pub async fn inspect_focused(&self) -> Option<TargetMetadata> {
        if let Some(ref mock) = self.mock_store {
            return mock.get_focused().map(|e| e.to_target_metadata());
        }

        if let Some(ref tx) = self.tx {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            if tx.send(UiaRequest::Focused(resp_tx)).await.is_err() {
                return None;
            }

            match tokio::time::timeout(Duration::from_millis(100), resp_rx).await {
                Ok(Ok(Some(info))) => Some(info.to_target_metadata()),
                Ok(Ok(None)) => None,
                Ok(Err(_)) => None,
                Err(_) => {
                    debug!("UIA focused query timed out (>100ms), falling back");
                    None
                }
            }
        } else {
            None
        }
    }
}
