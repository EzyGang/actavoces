from pathlib import Path
from typing import Any, Literal, Protocol

from pydantic import BaseModel, Field

from app.core.pydantic_base import AppBaseModel


type TranscriptionProfile = Literal['conservative_vad']
DEFAULT_TRANSCRIPTION_PROFILE: TranscriptionProfile = 'conservative_vad'


class FasterWhisperSegment(Protocol):
    start: float
    end: float
    text: str
    words: list[FasterWhisperWord] | None


class FasterWhisperWord(Protocol):
    start: float
    end: float
    word: str
    probability: float | None


class FasterWhisperInfo(Protocol):
    language: str | None


class FasterWhisperModel(Protocol):
    def transcribe(
        self,
        audio_path: str,
        **kwargs: Any,
    ) -> tuple[list[FasterWhisperSegment], FasterWhisperInfo]: ...


class FasterWhisperModelFactory(Protocol):
    def __call__(
        self,
        model_size_or_path: str,
        device: str = 'auto',
        device_index: int | list[int] = 0,
        compute_type: str = 'default',
        cpu_threads: int = 0,
        num_workers: int = 1,
        download_root: str | None = None,
        local_files_only: bool = False,
        files: Any = None,
        revision: str | None = None,
        use_auth_token: str | bool | None = None,
        **model_kwargs: Any,
    ) -> Any: ...


class Segment(AppBaseModel):
    id: int | None = None
    start: float = 0
    end: float = 0
    text: str = ''


class TranscriptionWord(AppBaseModel):
    segment_id: int
    text: str = ''
    start: float = 0
    end: float = 0
    probability: float | None = None


class SpeakerLabeledWord(AppBaseModel):
    segment_id: int
    speaker: str = 'Speaker'
    text: str = ''
    start: float = 0
    end: float = 0
    probability: float | None = None


class SpeakerLabeledUtterance(AppBaseModel):
    speaker: str = 'Speaker'
    start: float = 0
    end: float = 0
    text: str = ''


class SpeakerTurn(AppBaseModel):
    speaker: str = 'Speaker'
    start: float = 0
    end: float = 0


class SummaryOutput(BaseModel):
    title: str = Field(description='Concise (max 48) title of the conversation', max_length=48)
    summary: str = Field(
        description=(
            'Summary of a conversation. No limits. Markdown format preffered. '
            'Overall information, action items, risks, and unanswered questions and so on.'
        )
    )


class NeedsSetupResult(AppBaseModel):
    status: Literal['needs_setup'] = 'needs_setup'
    payload: dict[str, Any]


class FailedResult(AppBaseModel):
    status: Literal['failed'] = 'failed'
    payload: dict[str, Any]


class ModelInstallCompleteResult(AppBaseModel):
    status: Literal['complete'] = 'complete'
    payload: dict[str, Any]


class TranscriptionCompleteResult(AppBaseModel):
    status: Literal['complete'] = 'complete'
    segments: list[Segment]
    words: list[TranscriptionWord] = Field(default_factory=list)
    language: str | None = None
    warning: str | None = None


class TranscriptionVadOptions(AppBaseModel):
    enabled: bool = True
    profile: TranscriptionProfile = DEFAULT_TRANSCRIPTION_PROFILE
    parameters: dict[str, int | float]


class TranscriptionMetadata(AppBaseModel):
    model: str
    language: str | None = None
    transcription_profile: TranscriptionProfile = DEFAULT_TRANSCRIPTION_PROFILE
    vad: TranscriptionVadOptions
    source_start: float | None = None
    source_end: float | None = None


class SummaryCompleteResult(AppBaseModel):
    status: Literal['complete'] = 'complete'
    title: str
    summary: str


class TranscribePayload(AppBaseModel):
    audio_path: Path
    output_directory: Path
    segments: list[Segment] | None = None
    title: str = ''
    model: str = 'medium'
    language: str | None = None
    transcription_context: str = ''
    compute_type: str = 'auto'
    model_storage_directory: Path | None = None
    transcription_profile: TranscriptionProfile = DEFAULT_TRANSCRIPTION_PROFILE


class DiarizePayload(AppBaseModel):
    audio_path: Path | None = None
    output_directory: Path
    segments: list[Segment] = Field(default_factory=list)
    words: list[TranscriptionWord] = Field(default_factory=list)
    turns: list[SpeakerTurn] = Field(default_factory=list)
    title: str = ''
    speaker_count_mode: str = 'automatic'
    exact_speakers: Any = None
    min_speakers: Any = None
    max_speakers: Any = None
    backend: str = 'pyannote'
    api_key: str = ''


class DiarizationCheckPayload(AppBaseModel):
    api_key: str = ''


class ModelsPayload(AppBaseModel):
    model: str = 'medium'
    compute_type: str = 'auto'
    model_storage_directory: Path | None = None


class SummarizePayload(AppBaseModel):
    output_directory: Path
    summary: str | None = None
    title: str | None = None
    provider_base_url: str = ''
    api_key: str = ''
    model: str = ''
    diarized_transcript_path: Path | None = None
    transcript_path: Path | None = None
    summary_prompt: str = ''


class ModelStatus(AppBaseModel):
    name: str
    installed: bool
    setup_required: bool
    dependency: str


class ModelsStatusPayload(AppBaseModel):
    models: list[ModelStatus]


class RuntimeCapabilitiesPayload(AppBaseModel):
    faster_whisper_available: bool
    cuda_available: bool
    cuda_error: str | None = None


class ModelInstallPayload(AppBaseModel):
    model: str
    model_storage_directory: str


class TranscribeCompletePayload(AppBaseModel):
    segments_path: str
    words_path: str
    transcript_path: str
    warning: str | None = None


class DiarizeCompletePayload(AppBaseModel):
    diarization_path: str
    transcript_path: str
    speaker_labeled_words_path: str | None = None
    speaker_labeled_utterances_path: str | None = None


class SummarySetupPayload(AppBaseModel):
    missing: list[str]
    provider: str


class SummaryCompletePayload(AppBaseModel):
    summary_path: str
    title: str
