import logging
from typing import Any

from app.dtos import Segment, SpeakerTurn


def single_speaker_turns(
    segments: list[Segment],
    speaker_count_mode: str,
    exact_speakers: Any,
) -> list[SpeakerTurn]:
    if speaker_count_mode != 'exact' or int_value(exact_speakers) != 1 or not segments:
        return []

    return [
        SpeakerTurn(
            speaker='Speaker 1',
            start=min(segment.start for segment in segments),
            end=max(segment.end for segment in segments),
        )
    ]


def int_value(value: Any) -> int | None:
    try:
        return int(str(value))
    except (TypeError, ValueError) as e:
        logging.error(f'Failed to convert value to int: {e}')
        return None


def diarization_dependency(backend: str) -> str:
    if backend == 'pyannote':
        return 'pyannote-audio'

    return 'nemo-toolkit'
