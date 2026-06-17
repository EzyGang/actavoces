use crate::diarization::SpeakerTurn;

pub(crate) const SMOOTHING_POLICY: &str = "diarization_turn_smoothing_v1";
pub(crate) const TINY_GAP_SECONDS: f64 = 0.25;
pub(crate) const SHORT_ISLAND_SECONDS: f64 = 0.45;
pub(crate) const BACKCHANNEL_PRESERVE_SECONDS: f64 = 0.55;
pub(crate) const RAPID_FLIP_WINDOW_SECONDS: f64 = 1.25;

#[must_use]
pub(crate) fn smooth_turns(turns: &[SpeakerTurn]) -> Vec<SpeakerTurn> {
    let mut smoothed = normalized_turns(turns);
    merge_same_speaker_tiny_gaps(&mut smoothed);

    loop {
        if !remove_short_speaker_island(&mut smoothed) {
            break;
        }
        merge_same_speaker_tiny_gaps(&mut smoothed);
    }

    smoothed
}

fn normalized_turns(turns: &[SpeakerTurn]) -> Vec<SpeakerTurn> {
    let mut normalized: Vec<SpeakerTurn> = turns
        .iter()
        .filter(|turn| {
            turn.start.is_finite()
                && turn.end.is_finite()
                && turn.end > turn.start
                && !turn.speaker.trim().is_empty()
        })
        .cloned()
        .collect();

    normalized.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.speaker.cmp(&right.speaker))
    });

    normalized
}

fn merge_same_speaker_tiny_gaps(turns: &mut Vec<SpeakerTurn>) {
    let mut index = 1;

    while index < turns.len() {
        if turns[index - 1].speaker != turns[index].speaker {
            index += 1;
            continue;
        }

        if turns[index].start - turns[index - 1].end > TINY_GAP_SECONDS {
            index += 1;
            continue;
        }

        turns[index - 1].end = turns[index - 1].end.max(turns[index].end);
        turns.remove(index);
    }
}

fn remove_short_speaker_island(turns: &mut Vec<SpeakerTurn>) -> bool {
    if turns.len() < 3 {
        return false;
    }

    let mut index = 1;

    while index + 1 < turns.len() {
        let previous = &turns[index - 1];
        let current = &turns[index];
        let next = &turns[index + 1];

        if !is_short_island(previous, current, next) {
            index += 1;
            continue;
        }

        turns[index - 1].end = previous.end.max(current.end).max(next.end);
        turns.remove(index + 1);
        turns.remove(index);

        return true;
    }

    false
}

fn is_short_island(previous: &SpeakerTurn, current: &SpeakerTurn, next: &SpeakerTurn) -> bool {
    if previous.speaker != next.speaker || previous.speaker == current.speaker {
        return false;
    }

    if current.end - current.start >= SHORT_ISLAND_SECONDS {
        return false;
    }

    let surrounding_window = next.start - previous.end;

    surrounding_window <= RAPID_FLIP_WINDOW_SECONDS
}
