use crate::diarization::{single_speaker_turns, SpeakerTurn, TranscriptSegment};

#[test]
fn single_speaker_turn_covers_transcript_segments() {
    let turns = single_speaker_turns(&[
        TranscriptSegment {
            start: 2.0,
            end: 3.0,
            text: "Hello".to_owned(),
        },
        TranscriptSegment {
            start: 5.0,
            end: 8.0,
            text: "there".to_owned(),
        },
    ]);

    assert_eq!(
        turns,
        vec![SpeakerTurn {
            speaker: "Speaker 1".to_owned(),
            start: 2.0,
            end: 8.0,
        }]
    );
}
