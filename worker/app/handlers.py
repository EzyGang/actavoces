from collections.abc import Awaitable, Callable
from pathlib import Path
from typing import Any

from app.diarization import check_pyannote_setup, diarization_dependency, run_pyannote_diarization, single_speaker_turns
from app.diarization_smoothing import smoothing_metadata
from app.dtos import (
    DiarizationCheckPayload,
    DiarizationOutput,
    DiarizeCompletePayload,
    DiarizePayload,
    FailedResult,
    ModelsPayload,
    ModelsStatusPayload,
    ModelStatus,
    RuntimeCapabilitiesPayload,
    Segment,
    SpeakerLabeledUtterance,
    SpeakerLabeledWord,
    SpeakerTurn,
    SummarizePayload,
    SummaryCompletePayload,
    TranscribeCompletePayload,
    TranscribePayload,
    TranscriptionCompleteResult,
    TranscriptionMetadata,
    TranscriptionQualityMetadata,
    TranscriptionRepairAttempt,
    TranscriptionVadOptions,
    TranscriptionWord,
)
from app.events import command_event
from app.formatting import render_diarized_transcript, render_raw_transcript, render_summary
from app.json_utils import write_json, write_text
from app.models import (
    DEFAULT_MODEL,
    INITIAL_MODELS,
    TranscriptionResult,
    cuda_status,
    faster_whisper_available,
    install_faster_whisper_model,
    model_installed,
    run_faster_whisper,
    vad_parameters,
)
from app.protocol import WorkerCommand, WorkerEvent
from app.speaker_diarization import speaker_labeled_utterances, speaker_labeled_words
from app.summaries import run_openai_compatible_summary, summary_transcript
from app.transcription_quality import (
    MAX_RETRY_ATTEMPTS_PER_CHUNK,
    analyze_transcription_quality,
    risky_chunks_for_repair,
    skipped_repair_attempt,
)
from app.transcription_quality_chunks import load_source_chunks, merge_repaired_chunk, repair_chunk_by_id


type CommandHandler = Callable[[WorkerCommand], Awaitable[list[WorkerEvent]]]


async def handle(command: WorkerCommand) -> list[WorkerEvent]:
    handler = COMMAND_HANDLERS.get(command.name)

    if handler is None:
        return [command_event(command=command, name='command.unsupported', payload={'name': command.name})]

    return await handler(command)


async def handle_health_check(command: WorkerCommand) -> list[WorkerEvent]:
    return [command_event(command=command, name='health.ok', payload={'worker': 'actavoces-worker'})]


async def handle_models_status(command: WorkerCommand) -> list[WorkerEvent]:
    payload = ModelsPayload.model_validate(command.payload)
    dependency_ready = faster_whisper_available()
    models = [
        ModelStatus(
            name=model,
            installed=model_installed(model_name=model, storage_path=payload.model_storage_directory),
            setup_required=not dependency_ready,
            dependency='faster-whisper',
        )
        for model in INITIAL_MODELS
    ]

    return [command_event(command=command, name='models.status', payload=ModelsStatusPayload(models=models))]


async def handle_runtime_capabilities(command: WorkerCommand) -> list[WorkerEvent]:
    cuda_available, cuda_error = cuda_status()

    return [
        command_event(
            command=command,
            name='runtime.capabilities',
            payload=RuntimeCapabilitiesPayload(
                faster_whisper_available=faster_whisper_available(),
                cuda_available=cuda_available,
                cuda_error=cuda_error,
            ),
        )
    ]


async def handle_models_install(command: WorkerCommand) -> list[WorkerEvent]:
    payload = ModelsPayload.model_validate(command.payload)
    result = install_faster_whisper_model(
        model_name=payload.model or DEFAULT_MODEL,
        compute_type=payload.compute_type,
        model_storage_directory=payload.model_storage_directory,
    )

    if result.status == 'needs_setup':
        return [command_event(command=command, name='models.install.needs_setup', payload=result.payload)]

    if result.status == 'failed':
        return [command_event(command=command, name='command.failed', payload=result.payload)]

    return [
        command_event(command=command, name='models.install.progress', payload={'progress': 100}),
        command_event(command=command, name='models.install.complete', payload=result.payload),
    ]


async def handle_transcribe(command: WorkerCommand) -> list[WorkerEvent]:
    payload = TranscribePayload.model_validate(command.payload)

    if not payload.audio_path.exists():
        return [
            command_event(
                command=command,
                name='command.failed',
                payload={'error': f'Audio file does not exist: {payload.audio_path}'},
            )
        ]

    result = transcribe_audio(payload=payload)

    if result.status != 'complete':
        return transcription_failure_events(command=command, result=result)

    prepare_output_directories(output_directory=payload.output_directory)
    first_pass_segments = [segment.model_copy(deep=True) for segment in result.segments]
    first_pass_words = [word.model_copy(deep=True) for word in result.words]
    quality = run_transcription_quality_and_repair(payload=payload, result=result)
    write_first_pass_trace = any(attempt.status == 'repaired' for attempt in quality.repair_attempts)

    if write_first_pass_trace:
        write_json(
            first_pass_raw_segments_path(output_directory=payload.output_directory),
            {'segments': segment_payloads(first_pass_segments)},
        )
        write_json(
            first_pass_raw_words_path(output_directory=payload.output_directory),
            {'words': word_payloads(first_pass_words)},
        )
        quality.first_pass_raw_segments_path = str(
            first_pass_raw_segments_path(output_directory=payload.output_directory)
        )
        quality.first_pass_raw_words_path = str(first_pass_raw_words_path(output_directory=payload.output_directory))

    quality.final_raw_segments_path = str(raw_segments_path(output_directory=payload.output_directory))
    quality.final_raw_words_path = str(raw_words_path(output_directory=payload.output_directory))
    write_json(
        raw_segments_path(output_directory=payload.output_directory),
        {'segments': segment_payloads(segments=result.segments)},
    )
    write_json(
        raw_words_path(output_directory=payload.output_directory),
        {'words': word_payloads(words=result.words)},
    )
    write_text(
        raw_transcript_path(output_directory=payload.output_directory),
        render_raw_transcript(segments=result.segments, title=payload.title),
    )
    write_json(
        transcription_metadata_path(output_directory=payload.output_directory),
        transcription_metadata(payload=payload, result=result).model_dump(by_alias=True),
    )
    write_json(
        transcription_quality_path(output_directory=payload.output_directory),
        quality.model_dump(by_alias=True),
    )

    warning = transcription_warning(result_warning=result.warning, quality=quality)
    return [
        command_event(command=command, name='transcribe.progress', payload={'progress': 100}),
        command_event(
            command=command,
            name='transcribe.complete',
            payload=TranscribeCompletePayload(
                segments_path=str(raw_segments_path(output_directory=payload.output_directory)),
                words_path=str(raw_words_path(output_directory=payload.output_directory)),
                transcript_path=str(raw_transcript_path(output_directory=payload.output_directory)),
                warning=warning,
            ),
        ),
    ]


async def handle_diarize(command: WorkerCommand) -> list[WorkerEvent]:
    payload = DiarizePayload.model_validate(command.payload)
    raw_turns: list[SpeakerTurn] = []
    turns = payload.turns or single_speaker_turns(
        segments=payload.segments,
        speaker_count_mode=payload.speaker_count_mode,
        exact_speakers=payload.exact_speakers,
    )

    if not turns and payload.backend == 'pyannote':
        result = run_pyannote_diarization(
            audio_path=payload.audio_path or mixed_audio_path(output_directory=payload.output_directory),
            api_key=payload.api_key,
            speaker_count_mode=payload.speaker_count_mode,
            exact_speakers=payload.exact_speakers,
            min_speakers=payload.min_speakers,
            max_speakers=payload.max_speakers,
        )

        if isinstance(result, DiarizationOutput):
            turns = result.turns
            raw_turns = result.raw_turns
        elif isinstance(result, FailedResult):
            return [command_event(command=command, name='command.failed', payload=result.payload)]
        else:
            return [
                command_event(command=command, name='diarize.progress', payload={'progress': 0}),
                command_event(command=command, name='diarize.needs_setup', payload=result.payload),
            ]

    if not turns:
        return [
            command_event(command=command, name='diarize.progress', payload={'progress': 0}),
            command_event(
                command=command,
                name='diarize.needs_setup',
                payload={'dependency': diarization_dependency(backend=payload.backend)},
            ),
        ]

    prepare_output_directories(output_directory=payload.output_directory)
    labeled_words = speaker_labeled_words(words=payload.words, turns=turns)
    utterances = speaker_labeled_utterances(words=labeled_words)
    write_json(
        diarization_path(output_directory=payload.output_directory),
        diarization_payload(turns=turns, raw_turns=raw_turns),
    )
    if labeled_words:
        write_json(
            speaker_labeled_words_path(output_directory=payload.output_directory),
            {'words': speaker_labeled_word_payloads(words=labeled_words)},
        )
        write_json(
            speaker_labeled_utterances_path(output_directory=payload.output_directory),
            {'utterances': speaker_labeled_utterance_payloads(utterances=utterances)},
        )
    write_text(
        diarized_transcript_path(output_directory=payload.output_directory),
        render_diarized_transcript(segments=payload.segments, turns=turns, title=payload.title, words=payload.words),
    )

    return [
        command_event(command=command, name='diarize.progress', payload={'progress': 100}),
        command_event(
            command=command,
            name='diarize.complete',
            payload=DiarizeCompletePayload(
                diarization_path=str(diarization_path(output_directory=payload.output_directory)),
                transcript_path=str(diarized_transcript_path(output_directory=payload.output_directory)),
                speaker_labeled_words_path=str(speaker_labeled_words_path(output_directory=payload.output_directory))
                if labeled_words
                else None,
                speaker_labeled_utterances_path=str(
                    speaker_labeled_utterances_path(output_directory=payload.output_directory)
                )
                if labeled_words
                else None,
            ),
        ),
    ]


async def handle_diarization_check(command: WorkerCommand) -> list[WorkerEvent]:
    payload = DiarizationCheckPayload.model_validate(command.payload)
    setup_error = check_pyannote_setup(api_key=payload.api_key)

    if setup_error is not None:
        return [command_event(command=command, name='diarization.needs_setup', payload=setup_error.payload)]

    return [command_event(command=command, name='diarization.ready', payload={})]


async def handle_summarize(command: WorkerCommand) -> list[WorkerEvent]:
    payload = SummarizePayload.model_validate(command.payload)
    title = payload.title or ''
    summary = payload.summary

    if not summary:
        result = await run_openai_compatible_summary(
            provider_base_url=payload.provider_base_url,
            api_key=payload.api_key,
            model=payload.model,
            transcript=summary_transcript(payload=payload),
            summary_prompt=payload.summary_prompt,
        )

        if result.status == 'needs_setup':
            return [command_event(command=command, name='summarize.needs_setup', payload=result.payload)]

        if result.status == 'failed':
            return [command_event(command=command, name='command.failed', payload=result.payload)]

        title = result.title
        summary = result.summary

    prepare_output_directories(output_directory=payload.output_directory)
    write_text(summary_path(output_directory=payload.output_directory), render_summary(summary=summary))

    return [
        command_event(
            command=command,
            name='summarize.complete',
            payload=SummaryCompletePayload(
                summary_path=str(summary_path(output_directory=payload.output_directory)),
                title=title,
            ),
        )
    ]


def transcribe_audio(payload: TranscribePayload) -> TranscriptionResult:
    if payload.segments is not None:
        return TranscriptionCompleteResult(segments=payload.segments)

    result = run_faster_whisper(
        audio_path=payload.audio_path,
        model_name=payload.model,
        language=payload.language,
        transcription_context=payload.transcription_context,
        compute_type=payload.compute_type,
        model_storage_directory=payload.model_storage_directory,
        transcription_profile=payload.transcription_profile,
    )

    return result


def run_transcription_quality_and_repair(
    payload: TranscribePayload,
    result: TranscriptionCompleteResult,
) -> TranscriptionQualityMetadata:
    quality = analyze_transcription_quality(
        segments=result.segments,
        words=result.words,
        output_directory=payload.output_directory,
        language=result.language,
        expected_language=payload.language,
        language_probability=getattr(result, 'language_probability', None),
    )
    chunks = load_source_chunks(output_directory=payload.output_directory)

    for chunk_quality in risky_chunks_for_repair(quality=quality):
        chunk = repair_chunk_by_id(chunks=chunks, chunk_id=chunk_quality.chunk_id)
        if chunk is None:
            quality.repair_attempts.append(
                skipped_repair_attempt(chunk=chunk_quality, reason='chunk_audio_unavailable', model=payload.model)
            )
            continue
        for attempt in range(1, MAX_RETRY_ATTEMPTS_PER_CHUNK + 1):
            repair_result = run_faster_whisper(
                audio_path=chunk.audio_path or payload.audio_path,
                model_name=payload.model,
                language=result.language or payload.language,
                transcription_context=repair_context(context=payload.transcription_context),
                compute_type=payload.compute_type,
                model_storage_directory=payload.model_storage_directory,
                transcription_profile=payload.transcription_profile,
            )
            quality.repair_attempts.append(
                apply_repair_attempt(
                    result=result,
                    repair_result=repair_result,
                    chunk=chunk,
                    attempt=attempt,
                    model=payload.model,
                )
            )

    repaired_quality = analyze_transcription_quality(
        segments=result.segments,
        words=result.words,
        output_directory=payload.output_directory,
        language=result.language,
        expected_language=payload.language,
        language_probability=getattr(result, 'language_probability', None),
    )
    repaired_quality.repair_attempts = quality.repair_attempts
    return repaired_quality


def apply_repair_attempt(
    result: TranscriptionCompleteResult,
    repair_result: TranscriptionResult,
    chunk: Any,
    attempt: int,
    model: str,
) -> TranscriptionRepairAttempt:
    if repair_result.status != 'complete':
        return TranscriptionRepairAttempt(
            chunk_id=chunk.chunk_id,
            attempt=attempt,
            status='failed',
            reason='repair_transcription_failed',
            model=model,
            audio_path=str(chunk.audio_path) if chunk.audio_path is not None else None,
        )
    if not valid_repair_result(repair_result=repair_result):
        return TranscriptionRepairAttempt(
            chunk_id=chunk.chunk_id,
            attempt=attempt,
            status='failed',
            reason='repair_timestamps_invalid',
            model=model,
            audio_path=str(chunk.audio_path) if chunk.audio_path is not None else None,
        )
    segments, words, before_ids, after_ids = merge_repaired_chunk(
        first_pass_segments=result.segments,
        first_pass_words=result.words,
        repair_result=repair_result,
        chunk=chunk,
    )
    result.segments = segments
    result.words = words
    return TranscriptionRepairAttempt(
        chunk_id=chunk.chunk_id,
        attempt=attempt,
        status='repaired',
        reason='weak_chunk_retranscribed',
        model=model,
        audio_path=str(chunk.audio_path) if chunk.audio_path is not None else None,
        segment_ids_before=before_ids,
        segment_ids_after=after_ids,
    )


def valid_repair_result(repair_result: TranscriptionCompleteResult) -> bool:
    return all(segment.end >= segment.start for segment in repair_result.segments) and all(
        word.end >= word.start for word in repair_result.words
    )


def repair_context(context: str) -> str:
    extra = 'Re-transcribe this chunk conservatively. Preserve spoken words and avoid hallucinated filler.'

    return f'{context}\n{extra}' if context.strip() else extra


def transcription_warning(result_warning: str | None, quality: TranscriptionQualityMetadata) -> str | None:
    warnings = [warning for warning in [result_warning, *quality.warnings] if warning]
    if quality.unrepaired_risky_regions:
        warnings.append('Some transcript regions remain risky after quality checks.')

    return ' '.join(warnings) or None


def transcription_metadata(payload: TranscribePayload, result: TranscriptionCompleteResult) -> TranscriptionMetadata:
    source_start, source_end = segment_source_timing(segments=result.segments)

    return TranscriptionMetadata(
        model=payload.model,
        language=result.language or payload.language,
        transcription_profile=payload.transcription_profile,
        vad=TranscriptionVadOptions(
            enabled=True,
            profile=payload.transcription_profile,
            parameters=vad_parameters(transcription_profile=payload.transcription_profile),
        ),
        source_start=source_start,
        source_end=source_end,
    )


def segment_source_timing(segments: list[Segment]) -> tuple[float | None, float | None]:
    if not segments:
        return None, None

    return min(segment.start for segment in segments), max(segment.end for segment in segments)


def transcription_failure_events(command: WorkerCommand, result: TranscriptionResult) -> list[WorkerEvent]:
    if result.status == 'needs_setup':
        return [
            command_event(command=command, name='transcribe.progress', payload={'progress': 0}),
            command_event(command=command, name='transcribe.needs_setup', payload=result.payload),
        ]

    if result.status == 'failed':
        return [command_event(command=command, name='command.failed', payload=result.payload)]

    return [command_event(command=command, name='command.failed', payload={'error': 'Transcription failed'})]


def segment_payloads(segments: list[Segment]) -> list[dict[str, int | float | str | None]]:
    return [segment.model_dump(exclude_none=True) for segment in segments]


def word_payloads(words: list[TranscriptionWord]) -> list[dict[str, int | float | str | None]]:
    return [word.model_dump() for word in words]


def turn_payloads(turns: list[SpeakerTurn]) -> list[dict[str, float | str]]:
    return [turn.model_dump() for turn in turns]


def diarization_payload(turns: list[SpeakerTurn], raw_turns: list[SpeakerTurn]) -> dict[str, Any]:
    payload: dict[str, Any] = {'turns': turn_payloads(turns=turns)}

    if raw_turns:
        payload['rawTurns'] = turn_payloads(turns=raw_turns)
        payload['smoothing'] = smoothing_metadata()

    return payload


def speaker_labeled_word_payloads(words: list[SpeakerLabeledWord]) -> list[dict[str, int | float | str | None]]:
    return [word.model_dump() for word in words]


def speaker_labeled_utterance_payloads(utterances: list[SpeakerLabeledUtterance]) -> list[dict[str, float | str]]:
    return [utterance.model_dump() for utterance in utterances]


def prepare_output_directories(output_directory: Path) -> None:
    meta_path(output_directory=output_directory).mkdir(parents=True, exist_ok=True)


def meta_path(output_directory: Path) -> Path:
    return output_directory / 'meta'


def mixed_audio_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'recording.wav'


def raw_segments_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'raw-segments.json'


def raw_words_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'raw-words.json'


def transcription_metadata_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'transcription.json'


def transcription_quality_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'transcription-quality.json'


def first_pass_raw_segments_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'raw-segments-first-pass.json'


def first_pass_raw_words_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'raw-words-first-pass.json'


def raw_transcript_path(output_directory: Path) -> Path:
    return output_directory / 'raw-transcript.md'


def diarization_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'diarization.json'


def speaker_labeled_words_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'speaker-labeled-words.json'


def speaker_labeled_utterances_path(output_directory: Path) -> Path:
    return meta_path(output_directory=output_directory) / 'speaker-labeled-utterances.json'


def diarized_transcript_path(output_directory: Path) -> Path:
    return output_directory / 'diarized-transcript.md'


def summary_path(output_directory: Path) -> Path:
    return output_directory / 'summary.md'


COMMAND_HANDLERS: dict[str, CommandHandler] = {
    'health.check': handle_health_check,
    'runtime.capabilities': handle_runtime_capabilities,
    'models.status': handle_models_status,
    'models.install': handle_models_install,
    'diarization.check': handle_diarization_check,
    'transcribe.run': handle_transcribe,
    'diarize.run': handle_diarize,
    'summarize.run': handle_summarize,
}
