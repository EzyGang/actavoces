from app.dtos import SpeakerTurn


SMOOTHING_POLICY = 'diarization_turn_smoothing_v1'
TINY_GAP_SECONDS = 0.25
SHORT_ISLAND_SECONDS = 0.45
BACKCHANNEL_PRESERVE_SECONDS = 0.55
RAPID_FLIP_WINDOW_SECONDS = 1.25


def smooth_turns(turns: list[SpeakerTurn]) -> list[SpeakerTurn]:
    smoothed = normalized_turns(turns=turns)
    merge_same_speaker_tiny_gaps(turns=smoothed)

    while remove_short_speaker_island(turns=smoothed):
        merge_same_speaker_tiny_gaps(turns=smoothed)

    return smoothed


def smoothing_metadata() -> dict[str, float | str]:
    return {
        'policy': SMOOTHING_POLICY,
        'tinyGapSeconds': TINY_GAP_SECONDS,
        'shortIslandSeconds': SHORT_ISLAND_SECONDS,
        'backchannelPreserveSeconds': BACKCHANNEL_PRESERVE_SECONDS,
        'rapidFlipWindowSeconds': RAPID_FLIP_WINDOW_SECONDS,
    }


def normalized_turns(turns: list[SpeakerTurn]) -> list[SpeakerTurn]:
    return sorted(
        [
            SpeakerTurn(speaker=turn.speaker, start=turn.start, end=turn.end)
            for turn in turns
            if turn.start < turn.end and turn.speaker.strip()
        ],
        key=lambda turn: (turn.start, turn.end, turn.speaker),
    )


def merge_same_speaker_tiny_gaps(turns: list[SpeakerTurn]) -> None:
    index = 1

    while index < len(turns):
        previous = turns[index - 1]
        current = turns[index]

        if previous.speaker != current.speaker or current.start - previous.end > TINY_GAP_SECONDS:
            index += 1
            continue

        previous.end = max(previous.end, current.end)
        turns.pop(index)


def remove_short_speaker_island(turns: list[SpeakerTurn]) -> bool:
    if len(turns) < 3:
        return False

    index = 1

    while index + 1 < len(turns):
        previous = turns[index - 1]
        current = turns[index]
        next_turn = turns[index + 1]

        if not is_short_island(previous=previous, current=current, next_turn=next_turn):
            index += 1
            continue

        previous.end = max(previous.end, current.end, next_turn.end)
        turns.pop(index + 1)
        turns.pop(index)

        return True

    return False


def is_short_island(previous: SpeakerTurn, current: SpeakerTurn, next_turn: SpeakerTurn) -> bool:
    if previous.speaker != next_turn.speaker or previous.speaker == current.speaker:
        return False

    if current.end - current.start >= SHORT_ISLAND_SECONDS:
        return False

    return next_turn.start - previous.end <= RAPID_FLIP_WINDOW_SECONDS
