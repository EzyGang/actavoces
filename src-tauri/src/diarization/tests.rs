use crate::artifacts::{
    diarized_transcript_path, speaker_labeled_utterances_path, speaker_labeled_words_path,
};
use crate::diarization::render::{
    render_diarized_transcript, speaker_labeled_utterances, speaker_labeled_words,
};
use crate::diarization::{
    render_speaker_labeled_utterances, run_single_speaker_diarization, single_speaker_turns,
    SpeakerLabeledUtterance, SpeakerTurn, TranscriptSegment, TranscriptWord,
};

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

#[test]
fn local_diarization_writes_speaker_labeled_artifacts() {
    let artifact_path = std::env::temp_dir().join("actavoces-speaker-labeled-local-diarization");
    let _ = std::fs::remove_dir_all(&artifact_path);

    run_single_speaker_diarization(
        &artifact_path,
        &[TranscriptSegment {
            start: 0.0,
            end: 1.0,
            text: "Hello".to_owned(),
        }],
        &[TranscriptWord {
            segment_id: 0,
            text: "Hello".to_owned(),
            start: 0.0,
            end: 1.0,
            probability: Some(0.95),
        }],
        "Planning Call",
    )
    .unwrap();

    assert!(speaker_labeled_words_path(&artifact_path).exists());
    assert!(speaker_labeled_utterances_path(&artifact_path).exists());
    assert!(
        std::fs::read_to_string(diarized_transcript_path(&artifact_path))
            .unwrap()
            .contains("[00:00 - 00:01] Hello")
    );
}

#[test]
fn sortformer_transcript_rendering_uses_existing_turn_shape() {
    let content = render_diarized_transcript(
        &[TranscriptSegment {
            start: 0.0,
            end: 3.0,
            text: "Hello there".to_owned(),
        }],
        &[turn("Speaker 1", 0.0, 4.0)],
        "Planning Call",
        &[],
    );

    assert!(content.contains("# Diarized transcript - Planning Call"));
    assert!(content.contains("## Speaker 1"));
    assert!(content.contains("[00:00 - 00:03] Hello there"));
}

#[test]
fn sortformer_transcript_rendering_preserves_partially_overlapping_segments() {
    let content = render_diarized_transcript(
        &[
            TranscriptSegment {
                start: 0.0,
                end: 3.0,
                text: "first sentence".to_owned(),
            },
            TranscriptSegment {
                start: 3.0,
                end: 6.0,
                text: "second sentence".to_owned(),
            },
            TranscriptSegment {
                start: 6.0,
                end: 9.0,
                text: "third sentence".to_owned(),
            },
        ],
        &[turn("Speaker 1", 0.5, 2.5), turn("Speaker 2", 3.5, 5.5)],
        "",
        &[],
    );

    assert!(content.contains("first sentence"));
    assert!(content.contains("second sentence"));
    assert!(content.contains("third sentence"));
}

#[test]
fn word_rendering_splits_mixed_speaker_words_inside_segment() {
    let words = speaker_labeled_words(
        &[
            word("Hello", 0.0, 0.5),
            word("yes", 1.0, 1.2),
            word("continue", 1.4, 2.0),
        ],
        &[
            turn("Speaker 1", 0.0, 0.8),
            turn("Speaker 2", 0.9, 1.3),
            turn("Speaker 1", 1.3, 3.0),
        ],
    );
    let utterances = speaker_labeled_utterances(&words);

    assert_eq!(utterances[0].speaker, "Speaker 1");
    assert_eq!(utterances[0].text, "Hello");
    assert_eq!(utterances[1].speaker, "Speaker 2");
    assert_eq!(utterances[1].text, "yes");
    assert_eq!(utterances[2].speaker, "Speaker 1");
    assert_eq!(utterances[2].text, "continue");
}

#[test]
fn word_rendering_keeps_short_backchannel_separate() {
    let words = speaker_labeled_words(
        &[
            word("I", 0.0, 0.2),
            word("think", 0.3, 0.6),
            word("yes", 0.7, 0.9),
            word("we", 1.0, 1.2),
            word("ship", 1.3, 1.6),
        ],
        &[turn("Speaker 1", 0.0, 2.0), turn("Speaker 2", 0.65, 0.95)],
    );
    let utterances = speaker_labeled_utterances(&words);

    assert_eq!(utterances[0].text, "I think");
    assert_eq!(utterances[1].speaker, "Speaker 2");
    assert_eq!(utterances[1].text, "yes");
    assert_eq!(utterances[2].text, "we ship");
}

#[test]
fn word_rendering_uses_nearest_turn_for_no_overlap_words() {
    let words = speaker_labeled_words(
        &[word("between", 5.0, 6.0)],
        &[turn("Speaker 1", 0.0, 1.0), turn("Speaker 2", 7.0, 8.0)],
    );

    assert_eq!(words[0].speaker, "Speaker 2");
}

#[test]
fn speaker_labeled_utterance_renderer_keeps_transcript_shape() {
    let content = render_speaker_labeled_utterances(
        &[SpeakerLabeledUtterance {
            speaker: "Speaker 1".to_owned(),
            start: 0.0,
            end: 1.0,
            text: "Hello".to_owned(),
        }],
        "Planning Call",
    );

    assert!(content.contains("# Diarized transcript - Planning Call"));
    assert!(content.contains("## Speaker 1"));
    assert!(content.contains("[00:00 - 00:01] Hello"));
}

fn word(text: &str, start: f64, end: f64) -> TranscriptWord {
    TranscriptWord {
        segment_id: 0,
        text: text.to_owned(),
        start,
        end,
        probability: None,
    }
}

fn turn(speaker: &str, start: f64, end: f64) -> SpeakerTurn {
    SpeakerTurn {
        speaker: speaker.to_owned(),
        start,
        end,
    }
}
