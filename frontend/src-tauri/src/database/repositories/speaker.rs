// Named speakers for ML-diarized meetings ("speaker_00" -> "Anna").

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Error as SqlxError, FromRow, SqlitePool};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MeetingSpeaker {
    pub meeting_id: String,
    pub speaker_key: String,
    pub display_name: String,
}

pub struct SpeakersRepository;

impl SpeakersRepository {
    /// All named speakers for a meeting. Diarization keys with no row here
    /// yet simply have no custom name — the frontend falls back to a
    /// generic "Speaker N" label derived from the key itself.
    pub async fn get_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<MeetingSpeaker>, SqlxError> {
        sqlx::query_as::<_, MeetingSpeaker>(
            "SELECT meeting_id, speaker_key, display_name FROM meeting_speakers WHERE meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await
    }

    /// Set (or rename) a speaker's display name for a meeting.
    pub async fn rename(
        pool: &SqlitePool,
        meeting_id: &str,
        speaker_key: &str,
        display_name: &str,
    ) -> Result<(), SqlxError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO meeting_speakers (meeting_id, speaker_key, display_name, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(meeting_id, speaker_key) DO UPDATE SET
                display_name = excluded.display_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(meeting_id)
        .bind(speaker_key)
        .bind(display_name)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }
}
