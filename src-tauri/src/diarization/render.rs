use crate::diarization::{SpeakerTurn, TranscriptSegment};

#[derive(Clone, Debug, PartialEq)]
struct DiarizedTextGroup {
    speaker: String,
    start: f64,
    end: f64,
    texts: Vec<String>,
}

pub(super) fn render_diarized_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurn],
    title: &str,
) -> String {
    let mut lines = vec![diarized_transcript_heading(title), String::new()];
    let mut groups: Vec<DiarizedTextGroup> = Vec::new();

    for segment in segments {
        let speaker = best_speaker_for_segment(segment, turns)
            .unwrap_or("Unknown speaker")
            .to_owned();

        match groups.last_mut() {
            Some(group) if group.speaker == speaker => {
                group.end = segment.end;
                group.texts.push(segment.text.trim().to_owned());
            }
            _ => groups.push(DiarizedTextGroup {
                speaker,
                start: segment.start,
                end: segment.end,
                texts: vec![segment.text.trim().to_owned()],
            }),
        }
    }

    for group in groups {
        lines.push(format!("## {}", group.speaker));
        lines.push(String::new());
        lines.push(
            format!(
                "[{} - {}] {}",
                format_artifact_timestamp(group.start),
                format_artifact_timestamp(group.end),
                group.texts.join(" ")
            )
            .trim()
            .to_owned(),
        );
        lines.push(String::new());
    }

    lines.join("\n")
}

fn diarized_transcript_heading(title: &str) -> String {
    match title.trim() {
        "" => "# Diarized transcript".to_owned(),
        title => format!("# Diarized transcript - {title}"),
    }
}

fn best_speaker_for_segment<'a>(
    segment: &TranscriptSegment,
    turns: &'a [SpeakerTurn],
) -> Option<&'a str> {
    let segment_midpoint = (segment.start + segment.end) / 2.0;

    turns
        .iter()
        .max_by(|left, right| {
            let left_score = speaker_turn_score(segment, segment_midpoint, left);
            let right_score = speaker_turn_score(segment, segment_midpoint, right);

            left_score.total_cmp(&right_score)
        })
        .map(|turn| turn.speaker.as_str())
}

fn speaker_turn_score(
    segment: &TranscriptSegment,
    segment_midpoint: f64,
    turn: &SpeakerTurn,
) -> f64 {
    let overlap = segment.end.min(turn.end) - segment.start.max(turn.start);

    if overlap > 0.0 {
        return overlap;
    }

    let turn_midpoint = (turn.start + turn.end) / 2.0;

    -((segment_midpoint - turn_midpoint).abs())
}

fn format_artifact_timestamp(value: f64) -> String {
    let total_seconds = value as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use crate::diarization::render::render_diarized_transcript;
    use crate::diarization::{SpeakerTurn, TranscriptSegment};

    #[test]
    fn sortformer_transcript_rendering_uses_existing_turn_shape() {
        let content = render_diarized_transcript(
            &[TranscriptSegment {
                start: 0.0,
                end: 3.0,
                text: "Hello there".to_owned(),
            }],
            &[SpeakerTurn {
                speaker: "Speaker 1".to_owned(),
                start: 0.0,
                end: 4.0,
            }],
            "Planning Call",
        );

        assert!(content.contains("# Diarized transcript - Planning Call"));
        assert!(content.contains("## Speaker 1"));
        assert!(content.contains("[00:00 - 00:03] Hello there"));
        assert!(content.contains("Hello there"));
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
            &[
                SpeakerTurn {
                    speaker: "Speaker 1".to_owned(),
                    start: 0.5,
                    end: 2.5,
                },
                SpeakerTurn {
                    speaker: "Speaker 2".to_owned(),
                    start: 3.5,
                    end: 5.5,
                },
            ],
            "",
        );

        assert!(content.contains("first sentence"));
        assert!(content.contains("second sentence"));
        assert!(content.contains("third sentence"));
    }
}
