use crate::database::manager::DatabaseManager;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct AppState {
    pub db_manager: DatabaseManager,
    /// Meeting IDs currently running speaker diarization, so a second
    /// "Detect Speakers" click (or a stale UI after navigating away and
    /// back) can't kick off a duplicate, concurrent ONNX job for the same
    /// meeting.
    pub diarizing_meetings: Mutex<HashSet<String>>,
}
