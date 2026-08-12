import ctypes
import sys
from pathlib import Path
from typing import Any, cast

import ctranslate2

from app.dtos import (
    DEFAULT_TRANSCRIPTION_PROFILE,
    FailedResult,
    FasterWhisperModelFactory,
    FasterWhisperWord,
    ModelInstallCompleteResult,
    ModelInstallPayload,
    NeedsSetupResult,
    Segment,
    TranscriptionCompleteResult,
    TranscriptionWord,
)


try:
    from faster_whisper import WhisperModel

    faster_whisper_model_factory: FasterWhisperModelFactory | None = WhisperModel
    is_faster_whisper_available = True
except ImportError:
    faster_whisper_model_factory = None
    is_faster_whisper_available = False


ctranslate2_module: Any = ctranslate2
INITIAL_MODELS = ['small', 'medium', 'large-v3', 'distil-large-v3']
DEFAULT_MODEL = 'small'
MAX_TRANSCRIPTION_CONTEXT_CHARS = 4000
# Keep this limit in sync with src-tauri/src/app/commands/pipeline.rs.
CONSERVATIVE_VAD_PARAMETERS: dict[str, int | float] = {
    'threshold': 0.5,
    'min_speech_duration_ms': 0,
    'min_silence_duration_ms': 2000,
    'speech_pad_ms': 400,
}
type ModelInstallResult = NeedsSetupResult | FailedResult | ModelInstallCompleteResult
type TranscriptionResult = NeedsSetupResult | FailedResult | TranscriptionCompleteResult


class ModelCacheKey:
    def __init__(
        self,
        model_factory: FasterWhisperModelFactory,
        model_name: str,
        compute_type: str,
        model_storage_directory: Path | None,
    ) -> None:
        self.model_factory = model_factory
        self.model_name = model_name
        self.compute_type = compute_type
        self.model_storage_directory = model_storage_directory

    def __eq__(self, other: Any) -> bool:
        return (
            isinstance(other, ModelCacheKey)
            and self.model_factory is other.model_factory
            and self.model_name == other.model_name
            and self.compute_type == other.compute_type
            and self.model_storage_directory == other.model_storage_directory
        )


cached_model_key: ModelCacheKey | None = None
cached_model: Any = None


def get_cached_model(
    model_class: FasterWhisperModelFactory,
    model_name: str,
    compute_type: str,
    model_storage_directory: Path | None,
) -> Any:
    global cached_model, cached_model_key

    key = ModelCacheKey(
        model_factory=model_class,
        model_name=model_name,
        compute_type=compute_type,
        model_storage_directory=model_storage_directory,
    )
    if key != cached_model_key:
        cached_model = model_class(
            model_name,
            **model_kwargs(compute_type=compute_type, storage_path=model_storage_directory),
        )
        cached_model_key = key

    return cached_model


def discard_cached_model() -> None:
    global cached_model, cached_model_key

    cached_model = None
    cached_model_key = None


def run_faster_whisper(
    audio_path: Path,
    model_name: str,
    language: str | None,
    compute_type: str,
    model_storage_directory: Path | None,
    transcription_context: str = '',
    transcription_profile: str = DEFAULT_TRANSCRIPTION_PROFILE,
    model_factory: FasterWhisperModelFactory | None = None,
) -> TranscriptionResult:
    model_class = model_factory or faster_whisper_model_factory

    if model_class is None:
        return NeedsSetupResult(payload={'dependency': 'faster-whisper', 'model': model_name})

    try:
        segments, words, detected_language = transcribe_with_model(
            model_class=model_class,
            model_name=model_name,
            audio_path=audio_path,
            language=language,
            transcription_context=transcription_context,
            compute_type=compute_type,
            model_storage_directory=model_storage_directory,
            transcription_profile=transcription_profile,
        )

        return TranscriptionCompleteResult(segments=segments, words=words, language=detected_language)
    except Exception as error:
        discard_cached_model()
        if compute_type != 'cpu' and cuda_library_error(error=error):
            try:
                segments, words, detected_language = transcribe_with_model(
                    model_class=model_class,
                    model_name=model_name,
                    audio_path=audio_path,
                    language=language,
                    transcription_context=transcription_context,
                    compute_type='cpu',
                    model_storage_directory=model_storage_directory,
                    transcription_profile=transcription_profile,
                )

                return TranscriptionCompleteResult(
                    segments=segments,
                    words=words,
                    language=detected_language,
                    warning='CUDA libraries are unavailable; CPU fallback was used.',
                )
            except Exception as fallback_error:
                return FailedResult(payload={'error': str(fallback_error), 'model': model_name})

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


def cuda_status() -> tuple[bool, str | None]:
    if faster_whisper_model_factory is None:
        return False, 'faster-whisper is not installed'

    try:
        if ctranslate2_module.get_cuda_device_count() <= 0:
            return False, 'No CUDA device was detected'

        compute_types: set[str] = ctranslate2_module.get_supported_compute_types('cuda')
    except Exception as error:
        return False, str(error)

    if not compute_types:
        return False, 'No CUDA compute types are supported'

    missing_libraries = missing_cuda_libraries()
    if missing_libraries:
        return False, f'Missing NVIDIA libraries: {", ".join(missing_libraries)}'

    return True, None


def missing_cuda_libraries() -> list[str]:
    if sys.platform == 'win32':
        return unloaded_libraries(names=['cublas64_12.dll', 'cudnn64_9.dll'])

    if sys.platform.startswith('linux'):
        return unloaded_libraries(names=['libcublas.so.12', 'libcudnn.so.9'])

    return []


def unloaded_libraries(names: list[str]) -> list[str]:
    missing: list[str] = []

    for name in names:
        try:
            ctypes.CDLL(name)
        except OSError:
            missing.append(name)

    return missing


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

    if compute_type == 'cpu':
        kwargs['device'] = 'cpu'
        kwargs['compute_type'] = 'int8'
    elif compute_type == 'cuda':
        kwargs['device'] = 'cuda'
        kwargs['compute_type'] = 'int8_float16'
    elif compute_type not in {'auto', 'metal'}:
        kwargs['compute_type'] = compute_type

    if storage_path is not None:
        kwargs['download_root'] = str(storage_path)

    return kwargs


def transcribe_kwargs(language: str | None, transcription_context: str, transcription_profile: str) -> dict[str, Any]:
    kwargs: dict[str, Any] = {
        'vad_filter': True,
        'vad_parameters': vad_parameters(transcription_profile=transcription_profile),
        'word_timestamps': True,
    }

    if language and language != 'auto':
        kwargs['language'] = language

    prompt = normalized_transcription_context(context=transcription_context)

    if prompt:
        kwargs['initial_prompt'] = prompt

    return kwargs


def normalized_transcription_context(context: str) -> str:
    seen: set[str] = set()
    entries: list[str] = []

    for line in context.splitlines():
        entry = line.strip()

        if not entry or entry in seen:
            continue

        seen.add(entry)
        entries.append(entry)

    return '\n'.join(entries)[:MAX_TRANSCRIPTION_CONTEXT_CHARS]


def vad_parameters(transcription_profile: str) -> dict[str, int | float]:
    return CONSERVATIVE_VAD_PARAMETERS.copy()


def transcribe_with_model(
    model_class: FasterWhisperModelFactory,
    model_name: str,
    audio_path: Path,
    language: str | None,
    transcription_context: str,
    compute_type: str,
    model_storage_directory: Path | None,
    transcription_profile: str,
) -> tuple[list[Segment], list[TranscriptionWord], str | None]:
    model = get_cached_model(
        model_class=model_class,
        model_name=model_name,
        compute_type=compute_type,
        model_storage_directory=model_storage_directory,
    )
    raw_segments, info = model.transcribe(
        str(audio_path),
        **transcribe_kwargs(
            language=language,
            transcription_context=transcription_context,
            transcription_profile=transcription_profile,
        ),
    )
    segments: list[Segment] = []
    words: list[TranscriptionWord] = []

    for index, segment in enumerate(raw_segments):
        segments.append(Segment(id=index, start=segment.start, end=segment.end, text=segment.text))
        segment_words = cast(list[FasterWhisperWord] | None, getattr(segment, 'words', None))

        for word in segment_words or []:
            words.append(
                TranscriptionWord(
                    segment_id=index,
                    text=word.word,
                    start=word.start,
                    end=word.end,
                    probability=getattr(word, 'probability', None),
                )
            )

    return segments, words, info.language


def cuda_library_error(error: Exception) -> bool:
    message = str(error).lower()
    cuda_terms = ['cublas', 'cudnn', 'cuda', 'cublas64', 'cudnn64']

    return any(term in message for term in cuda_terms)
