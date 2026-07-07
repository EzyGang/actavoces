from collections.abc import Sequence
from typing import Any

from app.dtos import Segment, SpeakerLabeledUtterance, SpeakerTurn, TranscriptionWord
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


def render_clean_transcript(
    segments: Sequence[SegmentInput],
    title: str = '',
    turns: Sequence[SpeakerTurnInput] | None = None,
    words: Sequence[TranscriptionWord] | None = None,
    utterances: Sequence[SpeakerLabeledUtterance] | None = None,
) -> str:
    if utterances is not None and len(utterances) > 0:
        return render_clean_speaker_transcript(utterances=utterances, title=title)

    if turns is not None and len(turns) > 0:
        if words is not None and len(words) > 0:
            labeled_utterances = speaker_labeled_utterances(words=speaker_labeled_words(words=words, turns=turns))
            return render_clean_speaker_transcript(utterances=labeled_utterances, title=title)

        return render_clean_turn_transcript(segments=segments, turns=turns, title=title)

    text = clean_paragraph([segment.text for segment in normalize_segments(segments=segments)])
    if not text and words is not None:
        text = clean_paragraph([word.text for word in words])

    return render_clean_body(lines=[text] if text else [], title=title)


def render_clean_speaker_transcript(utterances: Sequence[SpeakerLabeledUtterance], title: str = '') -> str:
    lines = [clean_transcript_heading(title=title), '']
    current_speaker = ''
    current_texts: list[str] = []

    for utterance in utterances:
        text = clean_paragraph([utterance.text])
        if not text:
            continue

        if current_speaker != utterance.speaker:
            append_clean_speaker_group(lines=lines, speaker=current_speaker, texts=current_texts)
            current_speaker = utterance.speaker
            current_texts = [text]
            continue

        current_texts.append(text)

    append_clean_speaker_group(lines=lines, speaker=current_speaker, texts=current_texts)

    return '\n'.join(lines)


def render_clean_turn_transcript(
    segments: Sequence[SegmentInput],
    turns: Sequence[SpeakerTurnInput],
    title: str = '',
) -> str:
    lines = [clean_transcript_heading(title=title), '']
    normalized_segments = normalize_segments(segments=segments)

    for turn in normalize_turns(turns=turns):
        text = clean_paragraph(segment_texts_in_turn(segments=normalized_segments, start=turn.start, end=turn.end))
        if not text:
            continue

        lines.append(f'## {turn.speaker}')
        lines.append('')
        lines.append(text)
        lines.append('')

    return '\n'.join(lines)


def render_clean_body(lines: list[str], title: str = '') -> str:
    output = [clean_transcript_heading(title=title), '']
    output.extend(lines)

    if lines:
        output.append('')

    return '\n'.join(output)


def append_clean_speaker_group(lines: list[str], speaker: str, texts: list[str]) -> None:
    text = clean_paragraph(texts)
    if not speaker or not text:
        return

    lines.append(f'## {speaker}')
    lines.append('')
    lines.append(text)
    lines.append('')


def clean_transcript_heading(title: str = '') -> str:
    heading = 'Clean transcript'
    if title.strip():
        heading = f'{heading} - {title.strip()}'

    return f'# {heading}'


def clean_paragraph(texts: Sequence[str]) -> str:
    return ' '.join(' '.join(text.split()) for text in texts if text.strip()).strip()


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
