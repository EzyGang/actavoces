from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Protocol

from app.dtos import (
    Segment,
    TranscribePayload,
    TranscriptionChunkMetadata,
    TranscriptionCompleteResult,
    TranscriptionWord,
)
from app.models import TranscriptionResult, run_faster_whisper
from app.transcription_chunk_audio import (
    LONG_AUDIO_CHUNK_SECONDS,
    ChunkPlan,
    chunk_plans,
    wav_duration,
    write_wav_chunk,
)
from app.transcription_stitching import (
    apply_chunk_ranges,
    chunk_context,
    chunk_metadata,
    combine_warnings,
    single_chunk_metadata,
    stitch_chunk,
)


class Transcriber(Protocol):
    def __call__(
        self,
        *,
        audio_path: Path,
        model_name: str,
        language: str | None,
        transcription_context: str,
        compute_type: str,
        model_storage_directory: Path | None,
        transcription_profile: str,
    ) -> TranscriptionResult: ...


def run_chunked_transcription(
    payload: TranscribePayload,
    transcriber: Transcriber = run_faster_whisper,
) -> TranscriptionResult:
    source_duration = wav_duration(audio_path=payload.audio_path)

    if source_duration is None or source_duration <= LONG_AUDIO_CHUNK_SECONDS:
        return run_single_chunk(payload=payload, source_duration=source_duration, transcriber=transcriber)

    try:
        return run_long_audio_chunks(payload=payload, source_duration=source_duration, transcriber=transcriber)
    except Exception as error:
        result = run_single_chunk(payload=payload, source_duration=source_duration, transcriber=transcriber)
        if result.status != 'complete':
            return result

        warning = f'Chunk extraction failed; one-shot transcription was used. {error}'
        result.warning = combine_warnings(first=result.warning, second=warning)
        result.chunks = single_chunk_metadata(
            payload=payload, result=result, source_duration=source_duration, warning=warning
        )
        return result


def run_single_chunk(
    payload: TranscribePayload,
    source_duration: float | None,
    transcriber: Transcriber,
) -> TranscriptionResult:
    result = transcriber(
        audio_path=payload.audio_path,
        model_name=payload.model,
        language=payload.language,
        transcription_context=payload.transcription_context,
        compute_type=payload.compute_type,
        model_storage_directory=payload.model_storage_directory,
        transcription_profile=payload.transcription_profile,
    )

    if result.status == 'complete':
        result.source_duration = source_duration
        result.chunks = single_chunk_metadata(payload=payload, result=result, source_duration=source_duration)

    return result


def run_long_audio_chunks(
    payload: TranscribePayload,
    source_duration: float,
    transcriber: Transcriber,
) -> TranscriptionResult:
    plans = chunk_plans(audio_path=payload.audio_path, source_duration=source_duration)
    stitched_segments: list[Segment] = []
    stitched_words: list[TranscriptionWord] = []
    metadata: list[TranscriptionChunkMetadata] = []
    detected_language: str | None = None
    warning: str | None = None

    with TemporaryDirectory(prefix='actavoces-chunks-') as temporary_directory:
        for plan in plans:
            chunk_path = Path(temporary_directory) / f'chunk-{plan.chunk_id}.wav'
            write_wav_chunk(
                source_path=payload.audio_path, target_path=chunk_path, start=plan.overlap_start, end=plan.overlap_end
            )
            context = chunk_context(base_context=payload.transcription_context, segments=stitched_segments)
            result = transcriber(
                audio_path=chunk_path,
                model_name=payload.model,
                language=payload.language,
                transcription_context=context,
                compute_type=payload.compute_type,
                model_storage_directory=payload.model_storage_directory,
                transcription_profile=payload.transcription_profile,
            )
            if result.status != 'complete':
                return result

            detected_language = detected_language or result.language
            warning = combine_warnings(first=warning, second=result.warning)
            append_stitched_chunk(
                payload=payload,
                plan=plan,
                result=result,
                context=context,
                stitched_segments=stitched_segments,
                stitched_words=stitched_words,
                metadata=metadata,
            )

    return TranscriptionCompleteResult(
        segments=stitched_segments,
        words=stitched_words,
        language=detected_language,
        warning=warning,
        chunks=metadata,
        source_duration=source_duration,
    )


def append_stitched_chunk(
    payload: TranscribePayload,
    plan: ChunkPlan,
    result: TranscriptionCompleteResult,
    context: str,
    stitched_segments: list[Segment],
    stitched_words: list[TranscriptionWord],
    metadata: list[TranscriptionChunkMetadata],
) -> None:
    segment_start = len(stitched_segments)
    word_start = len(stitched_words)
    stitch_chunk(
        plan=plan,
        result=result,
        stitched_segments=stitched_segments,
        stitched_words=stitched_words,
    )
    chunk = chunk_metadata(payload=payload, plan=plan, result=result, context=context)
    apply_chunk_ranges(
        chunk=chunk,
        segment_start=segment_start,
        segment_end=len(stitched_segments),
        word_start=word_start,
        word_end=len(stitched_words),
    )
    metadata.append(chunk)
