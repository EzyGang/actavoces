from collections import Counter

from app.dtos import (
    Segment,
    TranscriptionQualityThresholds,
    TranscriptionSegmentIssue,
    TranscriptionSegmentQuality,
    TranscriptionWord,
    WordConfidenceAggregate,
)
from app.transcription_quality_recording import empty_segment_quality, recording_issue


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
