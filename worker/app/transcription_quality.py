from collections import Counter
from pathlib import Path
from typing import Any, Literal, cast

from app.dtos import (
    Segment,
    TranscriptionChunkQuality,
    TranscriptionCompleteResult,
    TranscriptionQualityMetadata,
    TranscriptionQualityThresholds,
    TranscriptionRepairAttempt,
    TranscriptionSegmentIssue,
    TranscriptionSegmentQuality,
    TranscriptionWord,
    UnrepairedRiskyRegion,
    WordConfidenceAggregate,
)
from app.json_utils import loads


MAX_RETRY_ATTEMPTS_PER_CHUNK = 1
MAX_REPAIRED_CHUNKS_PER_RECORDING = 3
QUALITY_METADATA_FILENAMES = ['audio-chunks.json', 'chunks.json', 'recording-chunks.json']


class SourceChunk(TranscriptionChunkQuality):
    audio_path: Path | None = None


def analyze_transcription_quality(
    segments: list[Segment],
    words: list[TranscriptionWord],
    output_directory: Path,
    language: str | None,
    expected_language: str | None,
    language_probability: float | None,
) -> TranscriptionQualityMetadata:
    thresholds = TranscriptionQualityThresholds()
    chunks = load_source_chunks(output_directory=output_directory)
    segment_quality, low_confidence_words = segment_qualities(segments=segments, words=words, thresholds=thresholds)
    chunk_quality = chunk_qualities(chunks=chunks, segments=segments, segment_quality=segment_quality)
    warnings: list[str] = []

    if expected_language and expected_language != 'auto' and language and expected_language != language:
        if language_probability is None or language_probability >= thresholds.unexpected_language_probability:
            issue = recording_issue(
                code='unexpected_language_mismatch', message=f'Detected {language}, expected {expected_language}.'
            )
            segment_quality.append(empty_segment_quality(issue=issue))

    if risky_issue_count(segment_quality=segment_quality) and not chunks:
        warnings.append(
            'Transcript quality risks were detected, but chunk metadata was unavailable; repair was skipped.'
        )

    issue_counts = issue_counter(segment_quality=segment_quality, chunk_quality=chunk_quality)
    score = recording_score(issue_counts=issue_counts, segment_count=max(len(segments), 1))
    status = status_from_counts(issue_counts=issue_counts)

    return TranscriptionQualityMetadata(
        thresholds=thresholds,
        overall_status=status,
        recording_score=score,
        issue_counts=dict(issue_counts),
        language=language,
        expected_language=expected_language,
        language_probability=language_probability,
        chunks_available=bool(chunks),
        per_chunk_issues=chunk_quality,
        per_segment_issues=segment_quality,
        low_confidence_words=low_confidence_words,
        unrepaired_risky_regions=unrepaired_regions(chunk_quality=chunk_quality, segment_quality=segment_quality),
        warnings=warnings,
    )


def risky_chunks_for_repair(quality: TranscriptionQualityMetadata) -> list[TranscriptionChunkQuality]:
    chunks = [chunk for chunk in quality.per_chunk_issues if chunk.status == 'risky']

    return chunks[:MAX_REPAIRED_CHUNKS_PER_RECORDING]


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


def repair_chunk_by_id(chunks: list[SourceChunk], chunk_id: str) -> SourceChunk | None:
    return next((chunk for chunk in chunks if chunk.chunk_id == chunk_id and chunk.audio_path is not None), None)


def load_source_chunks(output_directory: Path) -> list[SourceChunk]:
    for filename in QUALITY_METADATA_FILENAMES:
        path = output_directory / 'meta' / filename
        if path.exists():
            return parse_source_chunks(payload=loads(path.read_text()), base_path=path.parent)

    return []


def skipped_repair_attempt(chunk: TranscriptionChunkQuality, reason: str, model: str) -> TranscriptionRepairAttempt:
    return TranscriptionRepairAttempt(chunk_id=chunk.chunk_id, attempt=0, status='skipped', reason=reason, model=model)


def segment_qualities(
    segments: list[Segment], words: list[TranscriptionWord], thresholds: TranscriptionQualityThresholds
) -> tuple[list[TranscriptionSegmentQuality], list[WordConfidenceAggregate]]:
    by_segment = words_by_segment(words=words)
    qualities: list[TranscriptionSegmentQuality] = []
    aggregates: list[WordConfidenceAggregate] = []

    for segment in segments:
        segment_words = by_segment.get(segment.id or 0, [])
        quality = base_segment_quality(segment=segment, words=segment_words)
        quality.issues = segment_issues(segment=segment, words=segment_words, quality=quality, thresholds=thresholds)
        qualities.append(quality)
        aggregates.extend(low_confidence_aggregates(words=segment_words, thresholds=thresholds, segment_id=segment.id))

    if len(''.join(segment.text for segment in segments).strip()) <= thresholds.near_empty_max_chars:
        issue = recording_issue(code='empty_or_near_empty_transcript', message='Transcript is empty or near-empty.')
        qualities.append(empty_segment_quality(issue=issue))

    return qualities, aggregates


def segment_issues(
    segment: Segment,
    words: list[TranscriptionWord],
    quality: TranscriptionSegmentQuality,
    thresholds: TranscriptionQualityThresholds,
) -> list[TranscriptionSegmentIssue]:
    issues: list[TranscriptionSegmentIssue] = []
    text = segment.text.strip()
    add_issue(issues=issues, segment=segment, code='missing_word_timestamps', when=bool(text and not words))
    add_issue(
        issues=issues,
        segment=segment,
        code='low_average_word_confidence',
        when=quality.average_word_probability is not None
        and quality.average_word_probability < thresholds.low_average_word_confidence,
    )
    add_issue(
        issues=issues, segment=segment, code='very_low_word_confidence_cluster', when=has_low_cluster(words, thresholds)
    )
    add_issue(
        issues=issues, segment=segment, code='repeated_text', when=repeated_text(text=text, thresholds=thresholds)
    )
    add_issue(
        issues=issues,
        segment=segment,
        code='very_long_asr_segment',
        when=quality.duration > thresholds.long_segment_duration,
    )
    add_issue(
        issues=issues,
        segment=segment,
        code='high_compression_ratio',
        when=segment.compression_ratio is not None and segment.compression_ratio > thresholds.high_compression_ratio,
    )
    add_issue(
        issues=issues,
        segment=segment,
        code='text_in_long_no_speech_region',
        when=bool(text)
        and quality.duration >= thresholds.long_no_speech_min_duration
        and segment.no_speech_prob is not None
        and segment.no_speech_prob >= thresholds.long_no_speech_probability,
    )
    return issues


def base_segment_quality(segment: Segment, words: list[TranscriptionWord]) -> TranscriptionSegmentQuality:
    probabilities = [word.probability for word in words if word.probability is not None]

    return TranscriptionSegmentQuality(
        segment_id=segment.id,
        start=segment.start,
        end=segment.end,
        duration=max(segment.end - segment.start, 0),
        generated_text_length=len(segment.text.strip()),
        average_word_probability=sum(probabilities) / len(probabilities) if probabilities else None,
        average_logprob=segment.avg_logprob,
        compression_ratio=segment.compression_ratio,
        no_speech_probability=segment.no_speech_prob,
        missing_word_timestamps=bool(segment.text.strip() and not words),
    )


def low_confidence_aggregates(
    words: list[TranscriptionWord], thresholds: TranscriptionQualityThresholds, segment_id: int | None
) -> list[WordConfidenceAggregate]:
    aggregates: list[WordConfidenceAggregate] = []
    cluster: list[TranscriptionWord] = []
    for word in words:
        if word.probability is not None and word.probability < thresholds.very_low_word_confidence:
            cluster.append(word)
            continue
        append_cluster(aggregates=aggregates, cluster=cluster, segment_id=segment_id, thresholds=thresholds)
        cluster = []
    append_cluster(aggregates=aggregates, cluster=cluster, segment_id=segment_id, thresholds=thresholds)
    return aggregates


def append_cluster(
    aggregates: list[WordConfidenceAggregate],
    cluster: list[TranscriptionWord],
    segment_id: int | None,
    thresholds: TranscriptionQualityThresholds,
) -> None:
    if len(cluster) < thresholds.very_low_word_cluster_size:
        return
    probabilities = [word.probability for word in cluster if word.probability is not None]
    aggregates.append(
        WordConfidenceAggregate(
            segment_id=segment_id,
            start=cluster[0].start,
            end=cluster[-1].end,
            word_count=len(cluster),
            average_probability=sum(probabilities) / len(probabilities) if probabilities else None,
            minimum_probability=min(probabilities) if probabilities else None,
        )
    )


def chunk_qualities(
    chunks: list[SourceChunk], segments: list[Segment], segment_quality: list[TranscriptionSegmentQuality]
) -> list[TranscriptionChunkQuality]:
    qualities: list[TranscriptionChunkQuality] = []
    for chunk in chunks:
        chunk_segments = [
            segment for segment in segments if overlaps(start=segment.start, end=segment.end, chunk=chunk)
        ]
        issues = [
            issue for quality in segment_quality for issue in quality.issues if overlaps(issue.start, issue.end, chunk)
        ]
        status = 'risky' if any(issue.severity == 'risky' for issue in issues) else 'warning' if issues else 'ok'
        qualities.append(
            TranscriptionChunkQuality(
                chunk_id=chunk.chunk_id,
                start=chunk.start,
                end=chunk.end,
                duration=chunk.duration,
                generated_text_length=sum(len(segment.text.strip()) for segment in chunk_segments),
                score=max(0, 1 - (len(issues) * 0.2)),
                status=status,
                issues=issues,
            )
        )
    return qualities


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


def add_issue(issues: list[TranscriptionSegmentIssue], segment: Segment, code: str, when: bool) -> None:
    if when:
        issues.append(
            TranscriptionSegmentIssue(
                code=code,
                severity='risky',
                segment_id=segment.id,
                start=segment.start,
                end=segment.end,
                message=code.replace('_', ' '),
            )
        )


def repeated_text(text: str, thresholds: TranscriptionQualityThresholds) -> bool:
    words = [word.lower().strip('.,!?;:') for word in text.split()]
    if len(words) < thresholds.repeated_text_min_words:
        return False
    return any(count >= thresholds.repeated_text_min_words for count in Counter(words).values())


def has_low_cluster(words: list[TranscriptionWord], thresholds: TranscriptionQualityThresholds) -> bool:
    count = 0
    for word in words:
        if word.probability is not None and word.probability < thresholds.very_low_word_confidence:
            count += 1
            if count >= thresholds.very_low_word_cluster_size:
                return True
            continue
        count = 0
    return False


def words_by_segment(words: list[TranscriptionWord]) -> dict[int, list[TranscriptionWord]]:
    grouped: dict[int, list[TranscriptionWord]] = {}
    for word in words:
        grouped.setdefault(word.segment_id, []).append(word)
    return grouped


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
    except TypeError, ValueError:
        return None


def recording_issue(code: str, message: str) -> TranscriptionSegmentIssue:
    return TranscriptionSegmentIssue(code=code, severity='risky', start=0, end=0, message=message)


def empty_segment_quality(issue: TranscriptionSegmentIssue) -> TranscriptionSegmentQuality:
    return TranscriptionSegmentQuality(start=0, end=0, duration=0, generated_text_length=0, issues=[issue])


def risky_issue_count(segment_quality: list[TranscriptionSegmentQuality]) -> int:
    return sum(1 for quality in segment_quality for issue in quality.issues if issue.severity == 'risky')


def issue_counter(
    segment_quality: list[TranscriptionSegmentQuality], chunk_quality: list[TranscriptionChunkQuality]
) -> Counter[str]:
    counter: Counter[str] = Counter()
    for quality in segment_quality:
        counter.update(issue.code for issue in quality.issues)
    for quality in chunk_quality:
        counter.update(issue.code for issue in quality.issues)
    return counter


def recording_score(issue_counts: Counter[str], segment_count: int) -> float:
    return max(0, round(1 - (sum(issue_counts.values()) / segment_count * 0.15), 3))


def status_from_counts(issue_counts: Counter[str]) -> Literal['ok', 'warning', 'risky']:
    if sum(issue_counts.values()) >= 2:
        return 'risky'
    if issue_counts:
        return 'warning'
    return 'ok'


def unrepaired_regions(
    chunk_quality: list[TranscriptionChunkQuality], segment_quality: list[TranscriptionSegmentQuality]
) -> list[UnrepairedRiskyRegion]:
    if chunk_quality:
        return [
            UnrepairedRiskyRegion(
                chunk_id=chunk.chunk_id,
                start=chunk.start,
                end=chunk.end,
                reason='risky_chunk_not_repaired',
                issue_codes=sorted({issue.code for issue in chunk.issues}),
            )
            for chunk in chunk_quality
            if chunk.status == 'risky'
        ]
    return [
        UnrepairedRiskyRegion(
            start=issue.start, end=issue.end, reason='chunk_metadata_unavailable', issue_codes=[issue.code]
        )
        for quality in segment_quality
        for issue in quality.issues
        if issue.severity == 'risky'
    ]
