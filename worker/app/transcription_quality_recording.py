from collections import Counter
from typing import Literal

from app.dtos import (
    TranscriptionChunkQuality,
    TranscriptionSegmentIssue,
    TranscriptionSegmentQuality,
    UnrepairedRiskyRegion,
)


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
