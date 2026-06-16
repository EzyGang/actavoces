from collections.abc import Sequence
from typing import Any

from app.dtos import Segment, SpeakerTurn, TranscriptionWord
from app.speaker_diarization import render_speaker_labeled_utterances, speaker_labeled_utterances, speaker_labeled_words


type SegmentInput = Segment | dict[str, Any]
type SpeakerTurnInput = SpeakerTurn | dict[str, Any]


def render_raw_transcript(segments: Sequence[SegmentInput], title: str = '') -> str:
    heading = 'Raw transcript'
    if title.strip():
        heading = f'{heading} - {title.strip()}'

    lines = [f'# {heading}', '']

    for segment in normalize_segments(segments=segments):
        text = segment.text.strip()
        lines.append(f'[{format_timestamp(segment.start)} - {format_timestamp(segment.end)}] {text}')

    lines.append('')

    return '\n'.join(lines)


def render_diarized_transcript(
    segments: Sequence[SegmentInput],
    turns: Sequence[SpeakerTurnInput],
    title: str = '',
    words: Sequence[TranscriptionWord] | None = None,
) -> str:
    if words is not None and len(words) > 0:
        utterances = speaker_labeled_utterances(words=speaker_labeled_words(words=words, turns=turns))
        return render_speaker_labeled_utterances(utterances=utterances, title=title)

    heading = 'Diarized transcript'
    if title.strip():
        heading = f'{heading} - {title.strip()}'

    lines = [f'# {heading}', '']
    normalized_segments = normalize_segments(segments=segments)

    for turn in normalize_turns(turns=turns):
        text = ' '.join(segment_texts_in_turn(segments=normalized_segments, start=turn.start, end=turn.end))
        lines.append(f'## {turn.speaker}')
        lines.append('')
        lines.append(f'[{format_timestamp(turn.start)} - {format_timestamp(turn.end)}] {text}'.strip())
        lines.append('')

    return '\n'.join(lines)


def render_summary(summary: str) -> str:
    return f'# Summary\n\n{summary.strip()}\n'


def segment_texts_in_turn(segments: list[Segment], start: float, end: float) -> list[str]:
    texts: list[str] = []

    for segment in segments:
        if segment.start >= start and segment.end <= end:
            texts.append(segment.text.strip())

    return texts


def normalize_segments(segments: Sequence[SegmentInput]) -> list[Segment]:
    normalized: list[Segment] = []

    for segment in segments:
        if isinstance(segment, Segment):
            normalized.append(segment)
        else:
            normalized.append(Segment.model_validate(segment))

    return normalized


def normalize_turns(turns: Sequence[SpeakerTurnInput]) -> list[SpeakerTurn]:
    normalized: list[SpeakerTurn] = []

    for turn in turns:
        if isinstance(turn, SpeakerTurn):
            normalized.append(turn)
        else:
            normalized.append(SpeakerTurn.model_validate(turn))

    return normalized


def format_timestamp(value: float | int) -> str:
    total_seconds = int(float(value))
    minutes = total_seconds // 60
    seconds = total_seconds % 60

    return f'{minutes:02d}:{seconds:02d}'
