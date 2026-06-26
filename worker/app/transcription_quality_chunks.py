from pathlib import Path
from typing import Any, cast

from app.dtos import Segment, TranscriptionChunkQuality, TranscriptionCompleteResult, TranscriptionWord
from app.json_utils import loads


QUALITY_METADATA_FILENAMES = ['audio-chunks.json', 'chunks.json', 'recording-chunks.json']


class SourceChunk(TranscriptionChunkQuality):
    audio_path: Path | None = None


def load_source_chunks(output_directory: Path) -> list[SourceChunk]:
    for filename in QUALITY_METADATA_FILENAMES:
        path = output_directory / 'meta' / filename
        if path.exists():
            return parse_source_chunks(payload=loads(path.read_text()), base_path=path.parent)

    return []


def parse_source_chunks(payload: Any, base_path: Path) -> list[SourceChunk]:
    raw_value: Any = cast(dict[str, Any], payload).get('chunks', payload) if isinstance(payload, dict) else payload
    chunks: list[SourceChunk] = []
    if not isinstance(raw_value, list):
        return chunks
    raw_chunks = cast(list[Any], raw_value)
    for index, raw_chunk in enumerate(raw_chunks):
        if not isinstance(raw_chunk, dict):
            continue
        chunk = source_chunk(raw_chunk=cast(dict[str, Any], raw_chunk), index=index, base_path=base_path)
        if chunk is not None:
            chunks.append(chunk)
    return chunks


def source_chunk(raw_chunk: dict[str, Any], index: int, base_path: Path) -> SourceChunk | None:
    start = safe_float(raw_chunk.get('sourceStart', raw_chunk.get('start')))
    end = safe_float(raw_chunk.get('sourceEnd', raw_chunk.get('end')))
    if start is None or end is None or end <= start:
        return None
    path_value = raw_chunk.get('audioPath', raw_chunk.get('chunkPath', raw_chunk.get('path')))
    audio_path = chunk_audio_path(path_value=path_value, base_path=base_path)
    return SourceChunk(
        chunk_id=str(raw_chunk.get('id', raw_chunk.get('chunkId', index))),
        start=start,
        end=end,
        duration=end - start,
        generated_text_length=0,
        score=1,
        audio_path=audio_path,
    )


def repair_chunk_by_id(chunks: list[SourceChunk], chunk_id: str) -> SourceChunk | None:
    return next((chunk for chunk in chunks if chunk.chunk_id == chunk_id and chunk.audio_path is not None), None)


def merge_repaired_chunk(
    first_pass_segments: list[Segment],
    first_pass_words: list[TranscriptionWord],
    repair_result: TranscriptionCompleteResult,
    chunk: SourceChunk,
) -> tuple[list[Segment], list[TranscriptionWord], list[int], list[int]]:
    before_ids = [
        segment.id or 0
        for segment in first_pass_segments
        if overlaps(start=segment.start, end=segment.end, chunk=chunk)
    ]
    kept_segments = [
        segment for segment in first_pass_segments if not overlaps(start=segment.start, end=segment.end, chunk=chunk)
    ]
    kept_words = [word for word in first_pass_words if not overlaps(start=word.start, end=word.end, chunk=chunk)]
    repaired_segments = [offset_segment(segment=segment, chunk=chunk) for segment in repair_result.segments]
    repaired_words = [offset_word(word=word, chunk=chunk) for word in repair_result.words]
    merged_segments = sorted([*kept_segments, *repaired_segments], key=lambda segment: (segment.start, segment.end))

    for index, segment in enumerate(merged_segments):
        segment.id = index

    id_by_time = {(segment.start, segment.end): segment.id for segment in merged_segments}
    for word in [*kept_words, *repaired_words]:
        word.segment_id = nearest_segment_id(word=word, segments=merged_segments, id_by_time=id_by_time)

    return (
        merged_segments,
        sorted([*kept_words, *repaired_words], key=lambda word: (word.start, word.end)),
        before_ids,
        [segment.id or 0 for segment in repaired_segments],
    )


def overlaps(start: float, end: float, chunk: TranscriptionChunkQuality) -> bool:
    return start < chunk.end and end > chunk.start


def offset_segment(segment: Segment, chunk: SourceChunk) -> Segment:
    return Segment(
        start=clamp_time(value=segment.start + chunk.start, lower=chunk.start, upper=chunk.end),
        end=clamp_time(value=segment.end + chunk.start, lower=chunk.start, upper=chunk.end),
        text=segment.text,
        avg_logprob=segment.avg_logprob,
        compression_ratio=segment.compression_ratio,
        no_speech_prob=segment.no_speech_prob,
    )


def offset_word(word: TranscriptionWord, chunk: SourceChunk) -> TranscriptionWord:
    return TranscriptionWord(
        segment_id=word.segment_id,
        text=word.text,
        start=clamp_time(value=word.start + chunk.start, lower=chunk.start, upper=chunk.end),
        end=clamp_time(value=word.end + chunk.start, lower=chunk.start, upper=chunk.end),
        probability=word.probability,
    )


def nearest_segment_id(
    word: TranscriptionWord, segments: list[Segment], id_by_time: dict[tuple[float, float], int | None]
) -> int:
    for segment in segments:
        if segment.start <= word.start and word.end <= segment.end:
            return segment.id or 0
    nearest = min(segments, key=lambda segment: abs(segment.start - word.start), default=None)
    segment_id = id_by_time.get((nearest.start, nearest.end), 0) if nearest is not None else 0

    return segment_id or 0


def clamp_time(value: float, lower: float, upper: float) -> float:
    return min(max(value, lower), upper)


def chunk_audio_path(path_value: Any, base_path: Path) -> Path | None:
    if not isinstance(path_value, str) or not path_value.strip():
        return None
    path = Path(path_value)
    if not path.is_absolute():
        path = base_path / path
    return path if path.exists() else None


def safe_float(value: Any) -> float | None:
    try:
        return float(value)
    except (TypeError, ValueError) as error:
        del error
        return None
