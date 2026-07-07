use crate::artifacts::{
    clean_transcript_path, diarization_path, diarized_transcript_path, meta_directory,
    speaker_labeled_utterances_path, speaker_labeled_words_path,
};
use crate::diarization::render::{
    render_diarized_transcript, speaker_labeled_utterances, speaker_labeled_words,
};
use crate::diarization::smoothing::smooth_turns;
use crate::diarization::{
    render_speaker_labeled_utterances, run_single_speaker_diarization, single_speaker_turns,
    write_diarization_output, SpeakerLabeledUtterance, SpeakerTurn, TranscriptSegment,
    TranscriptWord,
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

    let output = run_single_speaker_diarization(
        &artifact_path,
        &[
            TranscriptSegment {
                start: 0.0,
                end: 1.0,
                text: "Hello".to_owned(),
            },
            TranscriptSegment {
                start: 1.0,
                end: 2.0,
                text: "there".to_owned(),
            },
        ],
        &[
            TranscriptWord {
                segment_id: 0,
                text: "Hello".to_owned(),
                start: 0.0,
                end: 1.0,
                probability: Some(0.95),
            },
            TranscriptWord {
                segment_id: 1,
                text: "there".to_owned(),
                start: 1.0,
                end: 2.0,
                probability: Some(0.95),
            },
        ],
        "Planning Call",
    )
    .unwrap();

    assert!(speaker_labeled_words_path(&artifact_path).exists());
    assert!(speaker_labeled_utterances_path(&artifact_path).exists());
    assert!(
        std::fs::read_to_string(diarized_transcript_path(&artifact_path))
            .unwrap()
            .contains("[00:00 - 00:02] Hello there")
    );
    assert_eq!(
        output.clean_transcript_path,
        clean_transcript_path(&artifact_path)
    );
    let clean_transcript = std::fs::read_to_string(clean_transcript_path(&artifact_path)).unwrap();
    assert!(clean_transcript.starts_with("# Clean transcript - Planning Call"));
    assert!(clean_transcript.contains("## Speaker 1"));
    assert!(clean_transcript.contains("Hello there"));
    assert!(!clean_transcript.contains("[00:"));
    let diarization = std::fs::read_to_string(diarization_path(&artifact_path)).unwrap();
    assert!(!diarization.contains("rawTurns"));
    assert!(!diarization.contains("smoothing"));
}

#[test]
fn smoothing_merges_same_speaker_turns_across_tiny_gap() {
    assert_eq!(
        smooth_turns(&[turn("Speaker 1", 0.0, 1.0), turn("Speaker 1", 1.1, 2.0)]),
        vec![turn("Speaker 1", 0.0, 2.0)]
    );
}

#[test]
fn smoothing_removes_short_speaker_island() {
    assert_eq!(
        smooth_turns(&[
            turn("Speaker 1", 0.0, 1.0),
            turn("Speaker 2", 1.05, 1.25),
            turn("Speaker 1", 1.3, 2.0),
        ]),
        vec![turn("Speaker 1", 0.0, 2.0)]
    );
}

#[test]
fn smoothing_reduces_rapid_speaker_flips() {
    assert_eq!(
        smooth_turns(&[
            turn("Speaker 1", 0.0, 1.0),
            turn("Speaker 2", 1.01, 1.2),
            turn("Speaker 1", 1.21, 1.4),
            turn("Speaker 2", 1.41, 2.0),
        ]),
        vec![turn("Speaker 1", 0.0, 1.4), turn("Speaker 2", 1.41, 2.0)]
    );
}

#[test]
fn smoothing_preserves_legitimate_short_backchannel() {
    assert_eq!(
        smooth_turns(&[
            turn("Speaker 1", 0.0, 1.0),
            turn("Speaker 2", 1.05, 1.6),
            turn("Speaker 1", 1.65, 2.5),
        ]),
        vec![
            turn("Speaker 1", 0.0, 1.0),
            turn("Speaker 2", 1.05, 1.6),
            turn("Speaker 1", 1.65, 2.5),
        ]
    );
}

#[test]
fn sortformer_output_writes_smoothed_turns_and_raw_metadata() {
    let artifact_path = std::env::temp_dir().join("actavoces-smoothed-local-diarization");
    let _ = std::fs::remove_dir_all(&artifact_path);
    std::fs::create_dir_all(meta_directory(&artifact_path)).unwrap();
    let raw_turns = vec![
        turn("Speaker 1", 0.0, 1.0),
        turn("Speaker 2", 1.05, 1.25),
        turn("Speaker 1", 1.3, 2.0),
    ];
    let turns = smooth_turns(&raw_turns);

    write_diarization_output(
        &artifact_path,
        &[TranscriptSegment {
            start: 0.0,
            end: 2.0,
            text: "Hello yes continue".to_owned(),
        }],
        &[
            word("Hello", 0.0, 0.5),
            word("yes", 1.1, 1.2),
            word("continue", 1.4, 1.8),
        ],
        turns,
        Some(raw_turns),
        "Planning Call",
    )
    .unwrap();

    let artifact: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(diarization_path(&artifact_path)).unwrap())
            .unwrap();
    let transcript = std::fs::read_to_string(diarized_transcript_path(&artifact_path)).unwrap();
    let clean_transcript = std::fs::read_to_string(clean_transcript_path(&artifact_path)).unwrap();

    assert_eq!(artifact["turns"].as_array().unwrap().len(), 1);
    assert_eq!(artifact["rawTurns"].as_array().unwrap().len(), 3);
    assert_eq!(
        artifact["smoothing"]["policy"].as_str().unwrap(),
        "diarization_turn_smoothing_v1"
    );
    assert!(transcript.contains("## Speaker 1"));
    assert!(!transcript.contains("## Speaker 2"));
    assert!(clean_transcript.contains("## Speaker 1"));
    assert!(clean_transcript.contains("Hello yes continue"));
    assert!(!clean_transcript.contains("## Speaker 2"));
    assert!(!clean_transcript.contains("[00:"));
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
