use crate::diarization::{SpeakerTurn, TranscriptSegment, TranscriptWord};

const PAUSE_SPLIT_SECONDS: f64 = 1.0;
const PUNCTUATION_SPLIT_SECONDS: f64 = 0.3;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SpeakerLabeledWord {
    pub(crate) segment_id: usize,
    pub(crate) speaker: String,
    pub(crate) text: String,
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) probability: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SpeakerLabeledUtterance {
    pub(crate) speaker: String,
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq)]
struct DiarizedTextGroup {
    speaker: String,
    start: f64,
    end: f64,
    texts: Vec<String>,
}

pub(crate) fn render_diarized_transcript(
    segments: &[TranscriptSegment],
    turns: &[SpeakerTurn],
    title: &str,
    utterances: &[SpeakerLabeledUtterance],
) -> String {
    if !utterances.is_empty() {
        return render_speaker_labeled_utterances(utterances, title);
    }

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

pub(crate) fn speaker_labeled_words(
    words: &[TranscriptWord],
    turns: &[SpeakerTurn],
) -> Vec<SpeakerLabeledWord> {
    let mut sorted_words = words.to_vec();
    let mut sorted_turns = turns.to_vec();

    sorted_words.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
    });
    sorted_turns.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.speaker.cmp(&right.speaker))
    });

    sorted_words
        .iter()
        .map(|word| SpeakerLabeledWord {
            segment_id: word.segment_id,
            speaker: best_speaker_for_word(word, &sorted_turns)
                .unwrap_or("Unknown speaker")
                .to_owned(),
            text: word.text.clone(),
            start: word.start,
            end: word.end,
            probability: word.probability,
        })
        .collect()
}

pub(crate) fn speaker_labeled_utterances(
    words: &[SpeakerLabeledWord],
) -> Vec<SpeakerLabeledUtterance> {
    let mut utterances: Vec<SpeakerLabeledUtterance> = Vec::new();
    let mut sorted_words = words.to_vec();

    sorted_words.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
    });

    for word in sorted_words {
        let text = word.text.trim();
        if text.is_empty() {
            continue;
        }

        match utterances.last_mut() {
            Some(utterance) if !should_start_utterance(utterance, &word) => {
                utterance.end = word.end;
                utterance.text = format!("{} {text}", utterance.text);
            }
            _ => utterances.push(SpeakerLabeledUtterance {
                speaker: word.speaker,
                start: word.start,
                end: word.end,
                text: text.to_owned(),
            }),
        }
    }

    utterances
}

pub(crate) fn render_speaker_labeled_utterances(
    utterances: &[SpeakerLabeledUtterance],
    title: &str,
) -> String {
    let mut lines = vec![diarized_transcript_heading(title), String::new()];

    for utterance in utterances {
        lines.push(format!("## {}", utterance.speaker));
        lines.push(String::new());
        lines.push(
            format!(
                "[{} - {}] {}",
                format_artifact_timestamp(utterance.start),
                format_artifact_timestamp(utterance.end),
                utterance.text
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

fn best_speaker_for_word<'a>(word: &TranscriptWord, turns: &'a [SpeakerTurn]) -> Option<&'a str> {
    let word_midpoint = (word.start + word.end) / 2.0;

    turns
        .iter()
        .max_by(|left, right| {
            let left_score = word_turn_score(word, word_midpoint, left);
            let right_score = word_turn_score(word, word_midpoint, right);

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

fn word_turn_score(word: &TranscriptWord, word_midpoint: f64, turn: &SpeakerTurn) -> f64 {
    let overlap = word.end.min(turn.end) - word.start.max(turn.start);

    if overlap > 0.0 {
        return overlap;
    }

    let turn_midpoint = (turn.start + turn.end) / 2.0;

    -((word_midpoint - turn_midpoint).abs())
}

fn should_start_utterance(utterance: &SpeakerLabeledUtterance, word: &SpeakerLabeledWord) -> bool {
    let pause = word.start - utterance.end;

    if utterance.speaker != word.speaker {
        return true;
    }

    if pause > PAUSE_SPLIT_SECONDS {
        return true;
    }

    pause >= PUNCTUATION_SPLIT_SECONDS && ends_with_terminal_punctuation(&utterance.text)
}

fn ends_with_terminal_punctuation(value: &str) -> bool {
    value.trim_end().ends_with(['.', '?', '!'])
}

fn format_artifact_timestamp(value: f64) -> String {
    let total_seconds = value as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}
