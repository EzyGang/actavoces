import json
import sys
from importlib import import_module
from pathlib import Path
from typing import Any
from urllib import request

from pydantic import BaseModel

from app.protocol import WorkerCommand, WorkerEvent


INITIAL_MODELS = ['small.en', 'medium.en', 'large-v3', 'distil-large-v3']
DEFAULT_MODEL = 'medium.en'


class SummaryResponse(BaseModel):
    title: Any = ''
    summary: Any = ''


def emit(event: WorkerEvent) -> None:
    sys.stdout.write(f'{event.model_dump_json()}\n')
    sys.stdout.flush()


def event(command: WorkerCommand, name: str, payload: dict[str, Any] | None = None) -> WorkerEvent:
    return WorkerEvent(command_id=command.id, event=name, payload=payload or {})


def handle(command: WorkerCommand) -> list[WorkerEvent]:
    if command.name == 'health.check':
        return [event(command, 'health.ok', {'worker': 'actavoces-worker'})]

    if command.name == 'models.status':
        return [handle_models_status(command)]

    if command.name == 'models.install':
        return handle_models_install(command)

    if command.name == 'transcribe.run':
        return handle_transcribe(command)

    if command.name == 'diarize.run':
        return handle_diarize(command)

    if command.name == 'summarize.run':
        return handle_summarize(command)

    return [event(command, 'command.unsupported', {'name': command.name})]


def handle_models_status(command: WorkerCommand) -> WorkerEvent:
    model_storage_directory = command.payload.get('modelStorageDirectory')
    storage_path = Path(str(model_storage_directory)) if model_storage_directory else None
    dependency_ready = faster_whisper_available()

    return event(
        command,
        'models.status',
        {
            'models': [
                {
                    'name': model,
                    'installed': model_installed(model, storage_path),
                    'setupRequired': not dependency_ready,
                    'dependency': 'faster-whisper',
                }
                for model in INITIAL_MODELS
            ]
        },
    )


def handle_models_install(command: WorkerCommand) -> list[WorkerEvent]:
    model = str(command.payload.get('model', DEFAULT_MODEL))
    compute_type = str(command.payload.get('computeType', 'auto'))
    model_storage_directory = command.payload.get('modelStorageDirectory')
    result = install_faster_whisper_model(
        model_name=model,
        compute_type=compute_type,
        model_storage_directory=Path(str(model_storage_directory)) if model_storage_directory else None,
    )

    if result['status'] == 'needs_setup':
        return [event(command, 'models.install.needs_setup', result['payload'])]

    if result['status'] == 'failed':
        return [event(command, 'command.failed', result['payload'])]

    return [
        event(command, 'models.install.progress', {'progress': 100}),
        event(command, 'models.install.complete', result['payload']),
    ]


def handle_transcribe(command: WorkerCommand) -> list[WorkerEvent]:
    audio_path = Path(str(command.payload.get('audioPath', '')))
    output_directory = Path(str(command.payload.get('outputDirectory', '')))
    segments = command.payload.get('segments')
    model = str(command.payload.get('model', DEFAULT_MODEL))
    language = command.payload.get('language')
    compute_type = str(command.payload.get('computeType', 'auto'))
    model_storage_directory = command.payload.get('modelStorageDirectory')

    if not audio_path.exists():
        return [
            event(
                command,
                'command.failed',
                {'error': f'Audio file does not exist: {audio_path}'},
            )
        ]

    if segments is None:
        result = run_faster_whisper(
            audio_path=audio_path,
            model_name=model,
            language=str(language) if language else None,
            compute_type=compute_type,
            model_storage_directory=Path(str(model_storage_directory)) if model_storage_directory else None,
        )

        if result['status'] == 'needs_setup':
            return [
                event(command, 'transcribe.progress', {'progress': 0}),
                event(command, 'transcribe.needs_setup', result['payload']),
            ]

        if result['status'] == 'failed':
            return [event(command, 'command.failed', result['payload'])]

        segments = result['segments']

    output_directory.mkdir(parents=True, exist_ok=True)
    write_json(output_directory / 'raw-segments.json', {'segments': segments})
    write_text(output_directory / 'raw-transcript.md', render_raw_transcript(segments))

    return [
        event(command, 'transcribe.progress', {'progress': 100}),
        event(
            command,
            'transcribe.complete',
            {
                'segmentsPath': str(output_directory / 'raw-segments.json'),
                'transcriptPath': str(output_directory / 'raw-transcript.md'),
            },
        ),
    ]


def handle_diarize(command: WorkerCommand) -> list[WorkerEvent]:
    output_directory = Path(str(command.payload.get('outputDirectory', '')))
    segments = command.payload.get('segments', [])
    turns = command.payload.get('turns', [])

    if not turns:
        turns = single_speaker_turns(
            segments=segments,
            speaker_count_mode=str(command.payload.get('speakerCountMode', 'automatic')),
            exact_speakers=command.payload.get('exactSpeakers'),
        )

        if not turns:
            return [
                event(command, 'diarize.progress', {'progress': 0}),
                event(
                    command,
                    'diarize.needs_setup',
                    {'dependency': diarization_dependency(str(command.payload.get('backend', 'nemoWhisper')))},
                ),
            ]

    output_directory.mkdir(parents=True, exist_ok=True)
    write_json(output_directory / 'diarization.json', {'turns': turns})
    write_text(
        output_directory / 'diarized-transcript.md',
        render_diarized_transcript(segments, turns),
    )

    return [
        event(command, 'diarize.progress', {'progress': 100}),
        event(
            command,
            'diarize.complete',
            {
                'diarizationPath': str(output_directory / 'diarization.json'),
                'transcriptPath': str(output_directory / 'diarized-transcript.md'),
            },
        ),
    ]


def single_speaker_turns(
    segments: list[dict[str, Any]],
    speaker_count_mode: str,
    exact_speakers: Any,
) -> list[dict[str, Any]]:
    if speaker_count_mode != 'exact' or int_value(exact_speakers) != 1 or not segments:
        return []

    return [
        {
            'speaker': 'Speaker 1',
            'start': min(float(segment.get('start', 0)) for segment in segments),
            'end': max(float(segment.get('end', 0)) for segment in segments),
        }
    ]


def int_value(value: Any) -> int | None:
    try:
        return int(str(value))
    except TypeError, ValueError:
        return None


def diarization_dependency(backend: str) -> str:
    if backend == 'pyannote':
        return 'pyannote-audio'

    return 'nemo-toolkit'


def handle_summarize(command: WorkerCommand) -> list[WorkerEvent]:
    output_directory = Path(str(command.payload.get('outputDirectory', '')))
    summary = command.payload.get('summary')
    title = command.payload.get('title')

    if not summary:
        result = run_openai_compatible_summary(
            provider_base_url=str(command.payload.get('providerBaseUrl', '')),
            api_key=str(command.payload.get('apiKey', '')),
            model=str(command.payload.get('model', '')),
            transcript=summary_transcript(command.payload),
            summary_prompt=str(command.payload.get('summaryPrompt', '')),
            title_prompt=str(command.payload.get('titlePrompt', '')),
        )

        if result['status'] == 'needs_setup':
            return [event(command, 'summarize.needs_setup', result['payload'])]

        if result['status'] == 'failed':
            return [event(command, 'command.failed', result['payload'])]

        summary = result['summary']
        title = result['title']

    output_directory.mkdir(parents=True, exist_ok=True)
    write_text(output_directory / 'summary.md', render_summary(str(summary)))

    return [
        event(
            command,
            'summarize.complete',
            {'summaryPath': str(output_directory / 'summary.md'), 'title': title or ''},
        )
    ]


def render_raw_transcript(segments: list[dict[str, Any]]) -> str:
    lines = ['# Raw transcript', '']

    for segment in segments:
        start = segment.get('start', 0)
        end = segment.get('end', 0)
        text = str(segment.get('text', '')).strip()
        lines.append(f'[{format_timestamp(start)} - {format_timestamp(end)}] {text}')

    lines.append('')

    return '\n'.join(lines)


def render_diarized_transcript(segments: list[dict[str, Any]], turns: list[dict[str, Any]]) -> str:
    lines = ['# Diarized transcript', '']

    for turn in turns:
        speaker = str(turn.get('speaker', 'Speaker'))
        start = float(turn.get('start', 0))
        end = float(turn.get('end', 0))
        text = ' '.join(segment_texts_in_turn(segments, start, end))
        lines.append(f'## {speaker}')
        lines.append('')
        lines.append(f'[{format_timestamp(start)} - {format_timestamp(end)}] {text}'.strip())
        lines.append('')

    return '\n'.join(lines)


def render_summary(summary: str) -> str:
    return f'# Summary\n\n{summary.strip()}\n'


def assemble_summary_prompt(transcript: str, prompt: str) -> str:
    return f'{prompt.strip()}\n\nTranscript:\n{transcript.strip()}'


def summary_transcript(payload: dict[str, Any]) -> str:
    inline_transcript = payload.get('transcript')

    if inline_transcript:
        return str(inline_transcript)

    for key in ('diarizedTranscriptPath', 'transcriptPath'):
        raw_path = payload.get(key)

        if raw_path:
            path = Path(str(raw_path))

            if path.exists():
                return path.read_text(encoding='utf-8')

    return ''


def run_openai_compatible_summary(
    provider_base_url: str,
    api_key: str,
    model: str,
    transcript: str,
    summary_prompt: str,
    title_prompt: str,
    urlopen: Any = request.urlopen,
) -> dict[str, Any]:
    missing = [
        name
        for name, value in [
            ('providerBaseUrl', provider_base_url),
            ('apiKey', api_key),
            ('model', model),
            ('transcript', transcript),
            ('summaryPrompt', summary_prompt),
        ]
        if not value.strip()
    ]

    if missing:
        return {
            'status': 'needs_setup',
            'payload': {'missing': missing, 'provider': provider_base_url or 'OpenAI-compatible'},
        }

    try:
        response = post_chat_completion(
            provider_base_url=provider_base_url,
            api_key=api_key,
            model=model,
            prompt=assemble_summary_prompt(
                transcript,
                summary_response_prompt(summary_prompt, title_prompt),
            ),
            urlopen=urlopen,
        )
        parsed = parse_summary_response(response)

        return {'status': 'complete', **parsed}
    except Exception as error:
        return {'status': 'failed', 'payload': {'error': str(error)}}


def summary_response_prompt(summary_prompt: str, title_prompt: str) -> str:
    title_instruction = title_prompt.strip() or 'Create a concise title.'

    return f'{summary_prompt.strip()}\n\n{title_instruction}\n\nReturn JSON with string fields title and summary.'


def post_chat_completion(
    provider_base_url: str,
    api_key: str,
    model: str,
    prompt: str,
    urlopen: Any,
) -> str:
    body = json.dumps(
        {
            'model': model,
            'temperature': 0.2,
            'messages': [
                {
                    'role': 'system',
                    'content': 'You produce concise meeting notes as valid JSON.',
                },
                {'role': 'user', 'content': prompt},
            ],
        }
    ).encode('utf-8')
    http_request = request.Request(
        chat_completions_url(provider_base_url),
        data=body,
        headers={
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json',
        },
        method='POST',
    )

    with urlopen(http_request, timeout=60) as response:
        payload = json.loads(response.read().decode('utf-8'))

    return str(payload['choices'][0]['message']['content'])


def chat_completions_url(provider_base_url: str) -> str:
    normalized = provider_base_url.strip().rstrip('/')

    if normalized.endswith('/chat/completions'):
        return normalized

    return f'{normalized}/chat/completions'


def parse_summary_response(content: str) -> dict[str, str]:
    stripped = content.strip()

    try:
        parsed: Any = json.loads(stripped)
    except json.JSONDecodeError:
        return {'title': '', 'summary': stripped}

    if not isinstance(parsed, dict):
        return {'title': '', 'summary': stripped}

    response = SummaryResponse.model_validate(parsed)

    return {
        'title': str(response.title).strip(),
        'summary': str(response.summary).strip(),
    }


def run_faster_whisper(
    audio_path: Path,
    model_name: str,
    language: str | None,
    compute_type: str,
    model_storage_directory: Path | None,
    model_factory: Any | None = None,
) -> dict[str, Any]:
    try:
        model_class = model_factory or load_faster_whisper_model_class()
    except ImportError:
        return {
            'status': 'needs_setup',
            'payload': {'dependency': 'faster-whisper', 'model': model_name},
        }

    try:
        model_kwargs: dict[str, Any] = {}

        if compute_type != 'auto':
            model_kwargs['compute_type'] = compute_type

        if model_storage_directory is not None:
            model_kwargs['download_root'] = str(model_storage_directory)

        model = model_class(model_name, **model_kwargs)
        transcribe_kwargs: dict[str, Any] = {}

        if language and language != 'auto':
            transcribe_kwargs['language'] = language

        raw_segments, info = model.transcribe(str(audio_path), **transcribe_kwargs)
        segments = [
            {
                'id': index,
                'start': segment.start,
                'end': segment.end,
                'text': segment.text,
            }
            for index, segment in enumerate(raw_segments)
        ]

        return {
            'status': 'complete',
            'segments': segments,
            'language': getattr(info, 'language', None),
        }
    except Exception as error:
        return {'status': 'failed', 'payload': {'error': str(error), 'model': model_name}}


def install_faster_whisper_model(
    model_name: str,
    compute_type: str,
    model_storage_directory: Path | None,
    model_factory: Any | None = None,
) -> dict[str, Any]:
    try:
        model_class = model_factory or load_faster_whisper_model_class()
    except ImportError:
        return {
            'status': 'needs_setup',
            'payload': {'dependency': 'faster-whisper', 'model': model_name},
        }

    try:
        model_kwargs: dict[str, Any] = {}

        if compute_type != 'auto':
            model_kwargs['compute_type'] = compute_type

        if model_storage_directory is not None:
            model_storage_directory.mkdir(parents=True, exist_ok=True)
            model_kwargs['download_root'] = str(model_storage_directory)

        model_class(model_name, **model_kwargs)

        return {
            'status': 'complete',
            'payload': {
                'model': model_name,
                'modelStorageDirectory': str(model_storage_directory or ''),
            },
        }
    except Exception as error:
        return {'status': 'failed', 'payload': {'error': str(error), 'model': model_name}}


def load_faster_whisper_model_class() -> Any:
    module = import_module('faster_whisper')

    return module.WhisperModel


def faster_whisper_available() -> bool:
    try:
        load_faster_whisper_model_class()
        return True
    except ImportError:
        return False


def model_installed(model_name: str, storage_path: Path | None) -> bool:
    if storage_path is None:
        return False

    expected_names = [
        model_name,
        f'models--Systran--faster-whisper-{model_name}',
        f'faster-whisper-{model_name}',
    ]

    return any((storage_path / name).exists() for name in expected_names)


def segment_texts_in_turn(segments: list[dict[str, Any]], start: float, end: float) -> list[str]:
    texts: list[str] = []

    for segment in segments:
        segment_start = float(segment.get('start', 0))
        segment_end = float(segment.get('end', 0))

        if segment_start >= start and segment_end <= end:
            texts.append(str(segment.get('text', '')).strip())

    return texts


def format_timestamp(value: float | int) -> str:
    total_seconds = int(float(value))
    minutes = total_seconds // 60
    seconds = total_seconds % 60

    return f'{minutes:02d}:{seconds:02d}'


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(f'{json.dumps(value, indent=2)}\n', encoding='utf-8')


def write_text(path: Path, value: str) -> None:
    path.write_text(value, encoding='utf-8')


def main() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue

        for worker_event in handle(WorkerCommand.model_validate(json.loads(line))):
            emit(worker_event)


if __name__ == '__main__':
    main()
