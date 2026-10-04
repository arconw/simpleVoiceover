use crate::{session::AudioSession, storage::Store};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use tokio::sync::Notify;

#[derive(Clone)]
pub struct StudioState {
    pub store: Arc<Mutex<Store>>,
    pub audio: Arc<Mutex<AudioSession>>,
    pub shutdown: Arc<Notify>,
    pub connected: Arc<AtomicBool>,
    pub close_requested: Arc<Notify>,
}
impl StudioState {
    pub fn new(store: Store) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            audio: Arc::new(Mutex::new(AudioSession::new())),
            shutdown: Arc::new(Notify::new()),
            connected: Arc::new(AtomicBool::new(false)),
            close_requested: Arc::new(Notify::new()),
        }
    }
}
