use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use statichost_proto::{ErrKind, TunnelMsg};
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct ResponseHead {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub has_body: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum BodyEvent {
    Chunk(Bytes),
    End,
    Error(ErrKind, String),
}

pub struct PendingRequest {
    pub head_tx: Option<oneshot::Sender<Result<ResponseHead, (ErrKind, String)>>>,
    pub body_tx: mpsc::Sender<BodyEvent>,
}

pub struct TunnelHandle {
    pub ws_tx: mpsc::Sender<TunnelMsg>,
    pub pending: DashMap<Uuid, PendingRequest>,
}

impl TunnelHandle {
    pub fn new(_name: String, ws_tx: mpsc::Sender<TunnelMsg>) -> Arc<Self> {
        Arc::new(Self {
            ws_tx,
            pending: DashMap::new(),
        })
    }

    pub async fn send(&self, msg: TunnelMsg) -> Result<(), &'static str> {
        self.ws_tx.send(msg).await.map_err(|_| "tunnel closed")
    }
}

pub struct Registry {
    inner: RwLock<std::collections::HashMap<String, Arc<TunnelHandle>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub async fn register(
        &self,
        name: String,
        handle: Arc<TunnelHandle>,
    ) -> Result<(), &'static str> {
        let mut map = self.inner.write().await;
        if map.contains_key(&name) {
            return Err("name already taken");
        }
        map.insert(name, handle);
        Ok(())
    }

    pub async fn unregister(&self, name: &str, handle_id: usize) {
        let mut map = self.inner.write().await;
        if let Some(existing) = map.get(name) {
            if Arc::as_ptr(existing) as usize == handle_id {
                map.remove(name);
            }
        }
    }

    pub async fn get(&self, name: &str) -> Option<Arc<TunnelHandle>> {
        self.inner.read().await.get(name).cloned()
    }

    #[allow(dead_code)]
    pub async fn names(&self) -> Vec<String> {
        self.inner.read().await.keys().cloned().collect()
    }
}
