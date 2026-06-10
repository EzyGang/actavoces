from pathlib import Path
from typing import Any, Literal, Protocol

from pydantic import Field

from app.core.pydantic_base import AppBaseModel


class FasterWhisperSegment(Protocol):
    start: float
    end: float
    text: str


class FasterWhisperInfo(Protocol):
    language: str | None


class FasterWhisperModel(Protocol):
    def transcribe(
        self,
        audio_path: str,
        **kwargs: Any,
    ) -> tuple[list[FasterWhisperSegment], FasterWhisperInfo]: ...


class FasterWhisperModelFactory(Protocol):
    def __call__(self, model_name: str, **kwargs: Any) -> FasterWhisperModel: ...


class Segment(AppBaseModel):
    id: int | None = None
    start: float = 0
    end: float = 0
    text: str = ''


class SpeakerTurn(AppBaseModel):
    speaker: str = 'Speaker'
    start: float = 0
    end: float = 0


class SummaryOutput(AppBaseModel):
    title: str = ''
    summary: str = ''


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
    language: str | None = None


class SummaryCompleteResult(AppBaseModel):
    status: Literal['complete'] = 'complete'
    title: str
    summary: str


class TranscribePayload(AppBaseModel):
    audio_path: Path = Field(alias='audioPath')
    output_directory: Path = Field(alias='outputDirectory')
    segments: list[Segment] | None = None
    model: str = 'medium.en'
    language: str | None = None
    compute_type: str = Field(default='auto', alias='computeType')
    model_storage_directory: Path | None = Field(default=None, alias='modelStorageDirectory')


class DiarizePayload(AppBaseModel):
    output_directory: Path = Field(alias='outputDirectory')
    segments: list[Segment] = Field(default_factory=list)
    turns: list[SpeakerTurn] = Field(default_factory=list)
    speaker_count_mode: str = Field(default='automatic', alias='speakerCountMode')
    exact_speakers: Any = Field(default=None, alias='exactSpeakers')
    backend: str = 'nemoWhisper'


class ModelsPayload(AppBaseModel):
    model: str = 'medium.en'
    compute_type: str = Field(default='auto', alias='computeType')
    model_storage_directory: Path | None = Field(default=None, alias='modelStorageDirectory')


class SummarizePayload(AppBaseModel):
    output_directory: Path = Field(alias='outputDirectory')
    summary: str | None = None
    title: str | None = None
    provider_base_url: str = Field(default='', alias='providerBaseUrl')
    api_key: str = Field(default='', alias='apiKey')
    model: str = ''
    transcript: str | None = None
    diarized_transcript_path: Path | None = Field(default=None, alias='diarizedTranscriptPath')
    transcript_path: Path | None = Field(default=None, alias='transcriptPath')
    summary_prompt: str = Field(default='', alias='summaryPrompt')
    title_prompt: str = Field(default='', alias='titlePrompt')
