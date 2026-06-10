from pathlib import Path
from typing import Any

from app.dtos import (
    FailedResult,
    FasterWhisperModelFactory,
    ModelInstallCompleteResult,
    ModelInstallPayload,
    NeedsSetupResult,
    Segment,
    TranscriptionCompleteResult,
)


try:
    from faster_whisper import WhisperModel

    faster_whisper_model_factory: FasterWhisperModelFactory | None = WhisperModel
    is_faster_whisper_available = True
except ImportError:
    faster_whisper_model_factory = None
    is_faster_whisper_available = False


INITIAL_MODELS = ['small.en', 'medium.en', 'large-v3', 'distil-large-v3']
DEFAULT_MODEL = 'medium.en'
type ModelInstallResult = NeedsSetupResult | FailedResult | ModelInstallCompleteResult
type TranscriptionResult = NeedsSetupResult | FailedResult | TranscriptionCompleteResult


def run_faster_whisper(
    audio_path: Path,
    model_name: str,
    language: str | None,
    compute_type: str,
    model_storage_directory: Path | None,
    model_factory: FasterWhisperModelFactory | None = None,
) -> TranscriptionResult:
    model_class = model_factory or faster_whisper_model_factory

    if model_class is None:
        return NeedsSetupResult(payload={'dependency': 'faster-whisper', 'model': model_name})

    try:
        model = model_class(model_name, **model_kwargs(compute_type=compute_type, storage_path=model_storage_directory))
        raw_segments, info = model.transcribe(
            str(audio_path),
            **transcribe_kwargs(language=language),
        )
        segments = [
            Segment(id=index, start=segment.start, end=segment.end, text=segment.text)
            for index, segment in enumerate(raw_segments)
        ]

        return TranscriptionCompleteResult(segments=segments, language=info.language)
    except Exception as error:
        return FailedResult(payload={'error': str(error), 'model': model_name})


def install_faster_whisper_model(
    model_name: str,
    compute_type: str,
    model_storage_directory: Path | None,
    model_factory: FasterWhisperModelFactory | None = None,
) -> ModelInstallResult:
    model_class = model_factory or faster_whisper_model_factory

    if model_class is None:
        return NeedsSetupResult(payload={'dependency': 'faster-whisper', 'model': model_name})

    try:
        if model_storage_directory is not None:
            model_storage_directory.mkdir(parents=True, exist_ok=True)

        model_class(model_name, **model_kwargs(compute_type=compute_type, storage_path=model_storage_directory))

        return ModelInstallCompleteResult(
            payload=ModelInstallPayload(
                model=model_name,
                model_storage_directory=str(model_storage_directory or ''),
            ).model_dump(by_alias=True),
        )
    except Exception as error:
        return FailedResult(payload={'error': str(error), 'model': model_name})


def faster_whisper_available() -> bool:
    return is_faster_whisper_available


def model_installed(model_name: str, storage_path: Path | None) -> bool:
    if storage_path is None:
        return False

    expected_names = [
        model_name,
        f'models--Systran--faster-whisper-{model_name}',
        f'faster-whisper-{model_name}',
    ]

    return any((storage_path / name).exists() for name in expected_names)


def model_kwargs(compute_type: str, storage_path: Path | None) -> dict[str, Any]:
    kwargs: dict[str, Any] = {}

    if compute_type != 'auto':
        kwargs['compute_type'] = compute_type

    if storage_path is not None:
        kwargs['download_root'] = str(storage_path)

    return kwargs


def transcribe_kwargs(language: str | None) -> dict[str, Any]:
    if language and language != 'auto':
        return {'language': language}

    return {}
