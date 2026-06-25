import wave
from dataclasses import dataclass
from pathlib import Path


LONG_AUDIO_CHUNK_SECONDS = 20 * 60
CHUNK_OVERLAP_SECONDS = 5.0
BOUNDARY_SEARCH_SECONDS = 30.0
ANALYSIS_WINDOW_SECONDS = 0.1
SILENCE_RMS_RATIO = 0.08


@dataclass(frozen=True)
class ChunkPlan:
    chunk_id: int
    source_start: float
    source_end: float
    overlap_start: float
    overlap_end: float


def wav_duration(audio_path: Path) -> float | None:
    try:
        with wave.open(str(audio_path), 'rb') as audio:
            frame_rate = audio.getframerate()
            if frame_rate <= 0:
                return None

            return audio.getnframes() / frame_rate
    except wave.Error:
        return None


def chunk_plans(audio_path: Path, source_duration: float) -> list[ChunkPlan]:
    silence_ranges = detect_silence_ranges(audio_path=audio_path)
    plans: list[ChunkPlan] = []
    source_start = 0.0

    while source_start < source_duration:
        chunk_id = len(plans)
        target_end = min(source_duration, source_start + LONG_AUDIO_CHUNK_SECONDS)
        source_end = (
            source_duration
            if target_end >= source_duration
            else boundary_before(silences=silence_ranges, target=target_end)
        )
        if source_end <= source_start:
            source_end = target_end

        plans.append(
            ChunkPlan(
                chunk_id=chunk_id,
                source_start=source_start,
                source_end=source_end,
                overlap_start=chunk_overlap_start(source_start=source_start, chunk_id=chunk_id),
                overlap_end=chunk_overlap_end(source_end=source_end, source_duration=source_duration),
            )
        )
        source_start = source_end

    return plans


def chunk_overlap_start(source_start: float, chunk_id: int) -> float:
    if chunk_id == 0:
        return source_start

    return max(0.0, source_start - CHUNK_OVERLAP_SECONDS)


def chunk_overlap_end(source_end: float, source_duration: float) -> float:
    if source_end >= source_duration:
        return source_end

    return min(source_duration, source_end + CHUNK_OVERLAP_SECONDS)


def detect_silence_ranges(audio_path: Path) -> list[tuple[float, float]]:
    with wave.open(str(audio_path), 'rb') as audio:
        sample_width = audio.getsampwidth()
        frame_rate = audio.getframerate()
        frames_per_window = max(1, int(frame_rate * ANALYSIS_WINDOW_SECONDS))
        levels = audio_levels(audio=audio, frames_per_window=frames_per_window, sample_width=sample_width)

    if not levels:
        return []

    threshold = max(levels) * SILENCE_RMS_RATIO
    minimum_windows = max(1, int(2 / ANALYSIS_WINDOW_SECONDS))
    silence_ranges: list[tuple[float, float]] = []
    silence_start: int | None = None

    for index, level in enumerate(levels):
        if level <= threshold and silence_start is None:
            silence_start = index
        elif level > threshold and silence_start is not None:
            append_silence_range(silence_ranges, silence_start, index, minimum_windows, frames_per_window, frame_rate)
            silence_start = None

    if silence_start is not None:
        append_silence_range(silence_ranges, silence_start, len(levels), minimum_windows, frames_per_window, frame_rate)

    return silence_ranges


def audio_levels(audio: wave.Wave_read, frames_per_window: int, sample_width: int) -> list[int]:
    levels: list[int] = []

    while True:
        frames = audio.readframes(frames_per_window)
        if not frames:
            break

        levels.append(pcm_rms(frames=frames, sample_width=sample_width))

    return levels


def pcm_rms(frames: bytes, sample_width: int) -> int:
    if sample_width not in {1, 2, 4} or not frames:
        return 0

    total = 0
    count = 0

    for index in range(0, len(frames) - sample_width + 1, sample_width):
        sample = int.from_bytes(frames[index : index + sample_width], byteorder='little', signed=sample_width != 1)
        if sample_width == 1:
            sample -= 128

        total += sample * sample
        count += 1

    return int((total / count) ** 0.5) if count else 0


def append_silence_range(
    silence_ranges: list[tuple[float, float]],
    start_index: int,
    end_index: int,
    minimum_windows: int,
    frames_per_window: int,
    frame_rate: int,
) -> None:
    if end_index - start_index < minimum_windows:
        return

    silence_ranges.append((start_index * frames_per_window / frame_rate, end_index * frames_per_window / frame_rate))


def boundary_before(silences: list[tuple[float, float]], target: float) -> float:
    earliest = max(0.0, target - BOUNDARY_SEARCH_SECONDS)
    candidates = [(start + end) / 2 for start, end in silences if earliest <= (start + end) / 2 <= target]

    return max(candidates) if candidates else target


def write_wav_chunk(source_path: Path, target_path: Path, start: float, end: float) -> None:
    with wave.open(str(source_path), 'rb') as source:
        params = source.getparams()
        frame_rate = source.getframerate()
        start_frame = max(0, int(start * frame_rate))
        end_frame = min(source.getnframes(), int(end * frame_rate))
        source.setpos(start_frame)
        frames = source.readframes(max(0, end_frame - start_frame))

    with wave.open(str(target_path), 'wb') as target:
        target.setparams(params)
        target.writeframes(frames)
