from pathlib import Path

from app.dtos import (
    Segment,
    TranscriptionChunkQuality,
    TranscriptionQualityMetadata,
    TranscriptionQualityThresholds,
    TranscriptionRepairAttempt,
    TranscriptionWord,
)
from app.transcription_quality_chunk_scoring import chunk_qualities
from app.transcription_quality_chunks import load_source_chunks
from app.transcription_quality_recording import (
    empty_segment_quality,
    issue_counter,
    recording_issue,
    recording_score,
    risky_issue_count,
    status_from_counts,
    unrepaired_regions,
)
from app.transcription_quality_scoring import segment_qualities


MAX_RETRY_ATTEMPTS_PER_CHUNK = 1
MAX_REPAIRED_CHUNKS_PER_RECORDING = 3


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


def skipped_repair_attempt(chunk: TranscriptionChunkQuality, reason: str, model: str) -> TranscriptionRepairAttempt:
    return TranscriptionRepairAttempt(chunk_id=chunk.chunk_id, attempt=0, status='skipped', reason=reason, model=model)
