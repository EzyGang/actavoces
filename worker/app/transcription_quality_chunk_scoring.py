from app.dtos import Segment, TranscriptionChunkQuality, TranscriptionSegmentQuality
from app.transcription_quality_chunks import SourceChunk, overlaps


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
