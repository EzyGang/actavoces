from app.dtos import (
    Segment,
    TranscribePayload,
    TranscriptionChunkMetadata,
    TranscriptionCompleteResult,
    TranscriptionWord,
)
from app.models import MAX_TRANSCRIPTION_CONTEXT_CHARS, normalized_transcription_context
from app.transcription_chunk_audio import ChunkPlan


def chunk_context(base_context: str, segments: list[Segment]) -> str:
    transcript = ' '.join(segment.text.strip() for segment in segments if segment.text.strip())[
        -MAX_TRANSCRIPTION_CONTEXT_CHARS:
    ]

    return normalized_transcription_context(context=f'{base_context}\n{transcript}')


def stitch_chunk(
    plan: ChunkPlan,
    result: TranscriptionCompleteResult,
    stitched_segments: list[Segment],
    stitched_words: list[TranscriptionWord],
) -> None:
    segment_id_map: dict[int, int] = {}

    for segment in result.segments:
        start = segment.start + plan.overlap_start
        end = segment.end + plan.overlap_start
        if end <= plan.source_start or start >= plan.source_end:
            continue

        stitched_id = len(stitched_segments)
        original_id = segment.id if segment.id is not None else stitched_id
        segment_id_map[original_id] = stitched_id
        stitched_segments.append(
            Segment(
                id=stitched_id,
                start=max(start, plan.source_start),
                end=min(end, plan.source_end),
                text=segment.text,
            )
        )

    for word in result.words:
        start = word.start + plan.overlap_start
        end = word.end + plan.overlap_start
        if end <= plan.source_start or start >= plan.source_end:
            continue

        segment_id = segment_id_map.get(word.segment_id)
        if segment_id is None:
            continue

        stitched_words.append(
            TranscriptionWord(
                segment_id=segment_id,
                text=word.text,
                start=max(start, plan.source_start),
                end=min(end, plan.source_end),
                probability=word.probability,
            )
        )


def chunk_metadata(
    payload: TranscribePayload,
    plan: ChunkPlan,
    result: TranscriptionCompleteResult,
    context: str,
) -> TranscriptionChunkMetadata:
    source_start, source_end = result_timing(result=result, offset=plan.overlap_start)

    return TranscriptionChunkMetadata(
        chunk_id=plan.chunk_id,
        source_start=plan.source_start,
        source_end=plan.source_end,
        overlap_start=plan.overlap_start,
        overlap_end=plan.overlap_end,
        asr_output_start=source_start,
        asr_output_end=source_end,
        model=payload.model,
        language=result.language or payload.language,
        transcription_profile=payload.transcription_profile,
        context_length=len(context),
    )


def apply_chunk_ranges(
    chunk: TranscriptionChunkMetadata,
    segment_start: int,
    segment_end: int,
    word_start: int,
    word_end: int,
) -> None:
    if segment_end > segment_start:
        chunk.segment_id_start = segment_start
        chunk.segment_id_end = segment_end - 1

    if word_end > word_start:
        chunk.word_id_start = word_start
        chunk.word_id_end = word_end - 1


def result_timing(result: TranscriptionCompleteResult, offset: float) -> tuple[float | None, float | None]:
    if not result.segments:
        return None, None

    return min(segment.start for segment in result.segments) + offset, max(
        segment.end for segment in result.segments
    ) + offset


def single_chunk_metadata(
    payload: TranscribePayload,
    result: TranscriptionCompleteResult,
    source_duration: float | None,
    warning: str | None = None,
) -> list[TranscriptionChunkMetadata]:
    source_start = 0.0
    source_end = source_duration or result_timing(result=result, offset=0)[1] or 0.0
    asr_start, asr_end = result_timing(result=result, offset=0)
    chunk = TranscriptionChunkMetadata(
        chunk_id=0,
        source_start=source_start,
        source_end=source_end,
        overlap_start=source_start,
        overlap_end=source_end,
        asr_output_start=asr_start,
        asr_output_end=asr_end,
        model=payload.model,
        language=result.language or payload.language,
        transcription_profile=payload.transcription_profile,
        context_length=len(normalized_transcription_context(context=payload.transcription_context)),
        stitch_warnings=[warning] if warning else [],
    )
    apply_chunk_ranges(
        chunk=chunk,
        segment_start=0,
        segment_end=len(result.segments),
        word_start=0,
        word_end=len(result.words),
    )

    return [chunk]


def combine_warnings(first: str | None, second: str | None) -> str | None:
    if first and second:
        return f'{first} {second}'

    return first or second
