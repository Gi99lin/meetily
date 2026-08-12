use crate::database::manager::DatabaseManager;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

pub(crate) fn single_job_semaphore() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(1))
}

pub struct AppState {
    pub db_manager: DatabaseManager,
    /// Meeting IDs currently running speaker diarization, so a second
    /// "Detect Speakers" click (or a stale UI after navigating away and
    /// back) can't kick off a duplicate, concurrent ONNX job for the same
    /// meeting.
    pub diarizing_meetings: Mutex<HashSet<String>>,
    /// Offline diarization is CPU and memory-bandwidth intensive. Keep a
    /// single global worker even when different meetings are requested.
    pub diarization_permit: Arc<Semaphore>,
}

impl AppState {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self {
            db_manager,
            diarizing_meetings: Default::default(),
            diarization_permit: single_job_semaphore(),
        }
    }
}
