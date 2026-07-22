-- Add meeting_speakers table for naming ML-clustered speakers ("Speaker 1" -> "Anna")
-- speaker_key matches the value written to transcripts.speaker for diarized segments
-- (e.g. "speaker_00", "speaker_01", ...). Rows are created lazily on first rename;
-- an unrenamed speaker_key just falls back to a generic display label in the UI.
CREATE TABLE IF NOT EXISTS meeting_speakers (
    meeting_id TEXT NOT NULL,
    speaker_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, speaker_key),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_meeting_speakers_meeting_id ON meeting_speakers(meeting_id);
