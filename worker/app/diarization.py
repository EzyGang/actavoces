import importlib
import logging
import shutil
from pathlib import Path
from typing import Any, Protocol

from app.dtos import FailedResult, NeedsSetupResult, Segment, SpeakerTurn


PYANNOTE_PIPELINE = 'pyannote/speaker-diarization-community-1'


class PyannotePipelineFactory(Protocol):
    def from_pretrained(self, checkpoint: str, token: str) -> Any: ...


pyannote_pipeline_factory: PyannotePipelineFactory | None = None

try:
    pyannote_audio_module: Any = importlib.import_module('pyannote.audio')
    pyannote_pipeline_factory = pyannote_audio_module.Pipeline
except ImportError:
    pass


type DiarizationResult = NeedsSetupResult | FailedResult | list[SpeakerTurn]


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


def check_pyannote_setup(api_key: str) -> NeedsSetupResult | None:
    factory = pyannote_pipeline_factory

    if factory is None:
        return setup_required(dependency='pyannote.audio', message='pyannote.audio is not installed')

    if not api_key.strip():
        return setup_required(dependency='hugging-face-token', message='Hugging Face token is required')

    if shutil.which('ffmpeg') is None:
        return setup_required(dependency='ffmpeg', message='ffmpeg is required for pyannote.audio')

    try:
        factory.from_pretrained(PYANNOTE_PIPELINE, token=api_key)
    except Exception as error:
        return setup_required(dependency='pyannote-model-access', message=pyannote_error_message(error=error))

    return None


def run_pyannote_diarization(
    audio_path: Path,
    api_key: str,
    speaker_count_mode: str,
    exact_speakers: Any,
    min_speakers: Any,
    max_speakers: Any,
) -> DiarizationResult:
    setup_error = check_pyannote_setup(api_key=api_key)
    if setup_error is not None:
        return setup_error
    factory = pyannote_pipeline_factory

    if factory is None:
        return setup_required(dependency='pyannote.audio', message='pyannote.audio is not installed')

    if not audio_path.exists():
        return FailedResult(payload={'error': f'Audio file does not exist: {audio_path}'})

    try:
        pipeline = factory.from_pretrained(PYANNOTE_PIPELINE, token=api_key)
        output = pipeline(
            str(audio_path),
            **speaker_kwargs(speaker_count_mode, exact_speakers, min_speakers, max_speakers),
        )
    except Exception as error:
        return FailedResult(payload={'error': str(error)})

    return normalize_pyannote_turns(output=output)


def speaker_kwargs(
    speaker_count_mode: str,
    exact_speakers: Any,
    min_speakers: Any,
    max_speakers: Any,
) -> dict[str, int]:
    if speaker_count_mode == 'exact' and int_value(exact_speakers):
        return {'num_speakers': int_value(exact_speakers) or 0}

    if speaker_count_mode != 'range':
        return {}

    kwargs: dict[str, int] = {}
    min_value = int_value(min_speakers)
    max_value = int_value(max_speakers)

    if min_value is not None:
        kwargs['min_speakers'] = min_value
    if max_value is not None:
        kwargs['max_speakers'] = max_value

    return kwargs


def normalize_pyannote_turns(output: Any) -> list[SpeakerTurn]:
    diarization = getattr(output, 'speaker_diarization', output)
    speaker_names: dict[str, str] = {}
    turns: list[SpeakerTurn] = []

    for turn, _, speaker in diarization.itertracks(yield_label=True):
        speaker_id = str(speaker)
        speaker_names.setdefault(speaker_id, f'Speaker {len(speaker_names) + 1}')
        turns.append(
            SpeakerTurn(
                speaker=speaker_names[speaker_id],
                start=float(turn.start),
                end=float(turn.end),
            )
        )

    return turns


def setup_required(dependency: str, message: str) -> NeedsSetupResult:
    return NeedsSetupResult(payload={'dependency': dependency, 'message': message})


def pyannote_error_message(error: Exception) -> str:
    message = str(error)

    if 'gated' in message.lower() or '401' in message or '403' in message:
        return 'Accept the pyannote model terms on Hugging Face and check the token'

    return message


def int_value(value: Any) -> int | None:
    try:
        return int(str(value))
    except (TypeError, ValueError) as error:
        logging.error(f'Failed to convert value to int: {error}')
        return None


def diarization_dependency(backend: str) -> str:
    if backend == 'pyannote':
        return 'pyannote.audio'

    return backend
