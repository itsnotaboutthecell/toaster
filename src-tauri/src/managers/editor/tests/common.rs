//! Shared test helpers for editor/tests.

use super::super::*;

pub(super) fn make_words() -> Vec<Word> {
    vec![
        Word {
            text: "Hello".into(),
            start_us: 0,
            end_us: 1_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.95,
            speaker_id: 0,
        },
        Word {
            text: "world".into(),
            start_us: 1_000_000,
            end_us: 2_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.90,
            speaker_id: 0,
        },
        Word {
            text: "this".into(),
            start_us: 2_000_000,
            end_us: 3_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.85,
            speaker_id: 0,
        },
        Word {
            text: "is".into(),
            start_us: 3_000_000,
            end_us: 4_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.80,
            speaker_id: 1,
        },
        Word {
            text: "a".into(),
            start_us: 4_000_000,
            end_us: 5_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.75,
            speaker_id: 1,
        },
        Word {
            text: "test".into(),
            start_us: 5_000_000,
            end_us: 6_000_000,
            deleted: false,
            silenced: false,
            confidence: 0.70,
            speaker_id: 1,
        },
    ]
}
