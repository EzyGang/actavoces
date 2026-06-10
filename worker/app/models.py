from importlib import import_module
from pathlib import Path
from typing import Any

from app.dtos import (
    FailedResult,
    FasterWhisperModelFactory,
    ModelInstallCompleteResult,
    NeedsSetupResult,
    Segment,
    TranscriptionCompleteResult,
)


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
    try:
        model_class = model_factory or load_faster_whisper_model_class()
    except ImportError:
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
    try:
        model_class = model_factory or load_faster_whisper_model_class()
    except ImportError:
        return NeedsSetupResult(payload={'dependency': 'faster-whisper', 'model': model_name})

    try:
        if model_storage_directory is not None:
            model_storage_directory.mkdir(parents=True, exist_ok=True)

        model_class(model_name, **model_kwargs(compute_type=compute_type, storage_path=model_storage_directory))

        return ModelInstallCompleteResult(
            payload={'model': model_name, 'modelStorageDirectory': str(model_storage_directory or '')},
        )
    except Exception as error:
        return FailedResult(payload={'error': str(error), 'model': model_name})


def load_faster_whisper_model_class() -> FasterWhisperModelFactory:
    module: Any = import_module('faster_whisper')

    return module.WhisperModel


def faster_whisper_available() -> bool:
    try:
        load_faster_whisper_model_class()
        return True
    except ImportError:
        return False


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
