from collections.abc import Awaitable, Callable

from app.diarization import diarization_dependency, single_speaker_turns
from app.dtos import DiarizePayload, ModelsPayload, Segment, SpeakerTurn, SummarizePayload, TranscribePayload
from app.events import command_event
from app.formatting import render_diarized_transcript, render_raw_transcript, render_summary
from app.json_utils import write_json, write_text
from app.models import (
    DEFAULT_MODEL,
    INITIAL_MODELS,
    TranscriptionResult,
    faster_whisper_available,
    install_faster_whisper_model,
    model_installed,
    run_faster_whisper,
)
from app.protocol import WorkerCommand, WorkerEvent
from app.summaries import run_openai_compatible_summary, summary_transcript


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
        {
            'name': model,
            'installed': model_installed(model_name=model, storage_path=payload.model_storage_directory),
            'setupRequired': not dependency_ready,
            'dependency': 'faster-whisper',
        }
        for model in INITIAL_MODELS
    ]

    return [command_event(command=command, name='models.status', payload={'models': models})]


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

    segments = payload.segments if payload.segments is not None else transcribe_audio(payload=payload)

    if not isinstance(segments, list):
        return transcription_failure_events(command=command, result=segments)

    payload.output_directory.mkdir(parents=True, exist_ok=True)
    write_json(payload.output_directory / 'raw-segments.json', {'segments': segment_payloads(segments=segments)})
    write_text(payload.output_directory / 'raw-transcript.md', render_raw_transcript(segments=segments))

    return [
        command_event(command=command, name='transcribe.progress', payload={'progress': 100}),
        command_event(
            command=command,
            name='transcribe.complete',
            payload={
                'segmentsPath': str(payload.output_directory / 'raw-segments.json'),
                'transcriptPath': str(payload.output_directory / 'raw-transcript.md'),
            },
        ),
    ]


async def handle_diarize(command: WorkerCommand) -> list[WorkerEvent]:
    payload = DiarizePayload.model_validate(command.payload)
    turns = payload.turns or single_speaker_turns(
        segments=payload.segments,
        speaker_count_mode=payload.speaker_count_mode,
        exact_speakers=payload.exact_speakers,
    )

    if not turns:
        return [
            command_event(command=command, name='diarize.progress', payload={'progress': 0}),
            command_event(
                command=command,
                name='diarize.needs_setup',
                payload={'dependency': diarization_dependency(backend=payload.backend)},
            ),
        ]

    payload.output_directory.mkdir(parents=True, exist_ok=True)
    write_json(payload.output_directory / 'diarization.json', {'turns': turn_payloads(turns=turns)})
    write_text(
        payload.output_directory / 'diarized-transcript.md',
        render_diarized_transcript(segments=payload.segments, turns=turns),
    )

    return [
        command_event(command=command, name='diarize.progress', payload={'progress': 100}),
        command_event(
            command=command,
            name='diarize.complete',
            payload={
                'diarizationPath': str(payload.output_directory / 'diarization.json'),
                'transcriptPath': str(payload.output_directory / 'diarized-transcript.md'),
            },
        ),
    ]


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
            title_prompt=payload.title_prompt,
        )

        if result.status == 'needs_setup':
            return [command_event(command=command, name='summarize.needs_setup', payload=result.payload)]

        if result.status == 'failed':
            return [command_event(command=command, name='command.failed', payload=result.payload)]

        title = result.title
        summary = result.summary

    payload.output_directory.mkdir(parents=True, exist_ok=True)
    write_text(payload.output_directory / 'summary.md', render_summary(summary=summary))

    return [
        command_event(
            command=command,
            name='summarize.complete',
            payload={'summaryPath': str(payload.output_directory / 'summary.md'), 'title': title},
        )
    ]


def transcribe_audio(payload: TranscribePayload) -> list[Segment] | TranscriptionResult:
    result = run_faster_whisper(
        audio_path=payload.audio_path,
        model_name=payload.model,
        language=payload.language,
        compute_type=payload.compute_type,
        model_storage_directory=payload.model_storage_directory,
    )

    if result.status == 'complete':
        return result.segments

    return result


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
    return [segment.model_dump() for segment in segments]


def turn_payloads(turns: list[SpeakerTurn]) -> list[dict[str, float | str]]:
    return [turn.model_dump() for turn in turns]


COMMAND_HANDLERS: dict[str, CommandHandler] = {
    'health.check': handle_health_check,
    'models.status': handle_models_status,
    'models.install': handle_models_install,
    'transcribe.run': handle_transcribe,
    'diarize.run': handle_diarize,
    'summarize.run': handle_summarize,
}
