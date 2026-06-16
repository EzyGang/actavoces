from collections.abc import Sequence
from typing import Any

from app.dtos import SpeakerLabeledUtterance, SpeakerLabeledWord, SpeakerTurn, TranscriptionWord


type TranscriptionWordInput = TranscriptionWord | dict[str, Any]
type SpeakerTurnInput = SpeakerTurn | dict[str, Any]

PAUSE_SPLIT_SECONDS = 1.0
PUNCTUATION_SPLIT_SECONDS = 0.3
TERMINAL_PUNCTUATION = ('.', '?', '!')


def speaker_labeled_words(
    words: Sequence[TranscriptionWordInput],
    turns: Sequence[SpeakerTurnInput],
) -> list[SpeakerLabeledWord]:
    normalized_turns = normalize_turns(turns=turns)
    labeled: list[SpeakerLabeledWord] = []

    for word in normalize_words(words=words):
        labeled.append(
            SpeakerLabeledWord(
                segment_id=word.segment_id,
                speaker=best_speaker_for_word(word=word, turns=normalized_turns),
                text=word.text,
                start=word.start,
                end=word.end,
                probability=word.probability,
            )
        )

    return labeled


def speaker_labeled_utterances(words: Sequence[SpeakerLabeledWord]) -> list[SpeakerLabeledUtterance]:
    utterances: list[SpeakerLabeledUtterance] = []

    for word in sorted(words, key=lambda item: (item.start, item.end)):
        if not word.text.strip():
            continue

        if not utterances or should_start_utterance(previous=utterances[-1], word=word):
            utterances.append(
                SpeakerLabeledUtterance(
                    speaker=word.speaker,
                    start=word.start,
                    end=word.end,
                    text=word.text.strip(),
                )
            )
            continue

        utterances[-1].end = word.end
        utterances[-1].text = f'{utterances[-1].text} {word.text.strip()}'

    return utterances


def render_speaker_labeled_utterances(utterances: Sequence[SpeakerLabeledUtterance], title: str = '') -> str:
    heading = 'Diarized transcript'
    if title.strip():
        heading = f'{heading} - {title.strip()}'

    lines = [f'# {heading}', '']

    for utterance in utterances:
        lines.append(f'## {utterance.speaker}')
        lines.append('')
        lines.append(
            f'[{format_timestamp(utterance.start)} - {format_timestamp(utterance.end)}] {utterance.text}'.strip()
        )
        lines.append('')

    return '\n'.join(lines)


def should_start_utterance(previous: SpeakerLabeledUtterance, word: SpeakerLabeledWord) -> bool:
    pause = word.start - previous.end

    if previous.speaker != word.speaker:
        return True

    if pause > PAUSE_SPLIT_SECONDS:
        return True

    return pause >= PUNCTUATION_SPLIT_SECONDS and previous.text.rstrip().endswith(TERMINAL_PUNCTUATION)


def best_speaker_for_word(word: TranscriptionWord, turns: list[SpeakerTurn]) -> str:
    if not turns:
        return 'Unknown speaker'

    word_midpoint = (word.start + word.end) / 2

    return max(turns, key=lambda turn: turn_score(word=word, word_midpoint=word_midpoint, turn=turn)).speaker


def turn_score(word: TranscriptionWord, word_midpoint: float, turn: SpeakerTurn) -> float:
    overlap = min(word.end, turn.end) - max(word.start, turn.start)

    if overlap > 0:
        turn_duration = max(turn.end - turn.start, 0.001)
        return overlap + (1 / turn_duration / 1_000_000)

    turn_midpoint = (turn.start + turn.end) / 2

    return -(abs(word_midpoint - turn_midpoint))


def normalize_words(words: Sequence[TranscriptionWordInput]) -> list[TranscriptionWord]:
    return [word if isinstance(word, TranscriptionWord) else TranscriptionWord.model_validate(word) for word in words]


def normalize_turns(turns: Sequence[SpeakerTurnInput]) -> list[SpeakerTurn]:
    return [turn if isinstance(turn, SpeakerTurn) else SpeakerTurn.model_validate(turn) for turn in turns]


def format_timestamp(value: float | int) -> str:
    total_seconds = int(float(value))
    minutes = total_seconds // 60
    seconds = total_seconds % 60

    return f'{minutes:02d}:{seconds:02d}'
