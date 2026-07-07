use crate::diarization::render::SpeakerLabeledUtterance;
use crate::diarization::{SpeakerTurn, TranscriptSegment};

#[derive(Clone, Debug, PartialEq)]
struct CleanTextGroup {
    speaker: String,
    texts: Vec<String>,
}

pub(crate) fn render_clean_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurn],
    title: &str,
    utterances: &[SpeakerLabeledUtterance],
) -> String {
    if !utterances.is_empty() {
        return render_clean_transcript_from_utterances(utterances, title);
    }

    if !turns.is_empty() {
        return render_clean_transcript_from_turns(segments, turns, title);
    }

    render_clean_transcript_from_segments(segments, title)
}

pub(crate) fn render_clean_transcript_from_segments(
    segments: &[TranscriptSegment],
    title: &str,
) -> String {
    let mut lines = vec![clean_transcript_heading(title), String::new()];
    let text = segments
        .iter()
        .map(|segment| normalized_text(&segment.text))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if !text.is_empty() {
        lines.push(text);
        lines.push(String::new());
    }

    lines.join("\n")
}

pub(crate) fn render_clean_transcript_from_utterances(
    utterances: &[SpeakerLabeledUtterance],
    title: &str,
) -> String {
    let mut groups: Vec<CleanTextGroup> = Vec::new();

    for utterance in utterances {
        let text = normalized_text(&utterance.text);
        if text.is_empty() {
            continue;
        }

        match groups.last_mut() {
            Some(group) if group.speaker == utterance.speaker => group.texts.push(text),
            _ => groups.push(CleanTextGroup {
                speaker: utterance.speaker.clone(),
                texts: vec![text],
            }),
        }
    }

    render_clean_speaker_groups(groups, title)
}

fn render_clean_transcript_from_turns(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurn],
    title: &str,
) -> String {
    let mut groups: Vec<CleanTextGroup> = Vec::new();

    for segment in segments {
        let text = normalized_text(&segment.text);
        if text.is_empty() {
            continue;
        }

        let speaker = best_speaker_for_segment(segment, turns)
            .unwrap_or("Unknown speaker")
            .to_owned();
        match groups.last_mut() {
            Some(group) if group.speaker == speaker => group.texts.push(text),
            _ => groups.push(CleanTextGroup {
                speaker,
                texts: vec![text],
            }),
        }
    }

    render_clean_speaker_groups(groups, title)
}

fn render_clean_speaker_groups(groups: Vec<CleanTextGroup>, title: &str) -> String {
    let mut lines = vec![clean_transcript_heading(title), String::new()];

    for group in groups {
        let text = group.texts.join(" ");
        if text.is_empty() {
            continue;
        }

        lines.push(format!("## {}", group.speaker));
        lines.push(String::new());
        lines.push(text);
        lines.push(String::new());
    }

    lines.join("\n")
}

fn clean_transcript_heading(title: &str) -> String {
    match title.trim() {
        "" => "# Clean transcript".to_owned(),
        title => format!("# Clean transcript - {title}"),
    }
}

fn normalized_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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
        let turn_duration = (turn.end - turn.start).max(0.001);

        return overlap + (1.0 / turn_duration / 1_000_000.0);
    }

    let turn_midpoint = (turn.start + turn.end) / 2.0;

    -((segment_midpoint - turn_midpoint).abs())
}
