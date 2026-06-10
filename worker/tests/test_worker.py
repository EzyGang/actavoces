from pathlib import Path
from types import SimpleNamespace
from typing import Any

from pydantic_ai import Agent
from pydantic_ai.models.test import TestModel
from pytest_mock import MockerFixture

from app.dtos import SummarizePayload, SummaryOutput
from app.events import emit
from app.formatting import render_diarized_transcript, render_raw_transcript
from app.handlers import handle
from app.models import install_faster_whisper_model, model_installed, run_faster_whisper
from app.protocol import WorkerCommand, WorkerEvent
from app.summaries import assemble_summary_prompt, build_summary_agent, run_openai_compatible_summary


async def test_health_check_returns_ok_event() -> None:
    events = await handle(WorkerCommand(id='1', name='health.check'))

    assert events[0].event == 'health.ok'


def test_emit_serializes_worker_event_aliases(capsys: Any) -> None:
    emit(WorkerEvent(command_id='1', event='health.ok', payload={'worker': 'test'}))

    assert capsys.readouterr().out == '{"commandId":"1","event":"health.ok","payload":{"worker":"test"}}\n'


async def test_transcribe_run_writes_supplied_segments(tmp_path: Path) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')
    command = WorkerCommand(
        id='2',
        name='transcribe.run',
        payload={
            'audio_path': str(audio_path),
            'output_directory': str(tmp_path),
            'segments': [{'start': 0, 'end': 3, 'text': ' Hello '}],
        },
    )

    events = await handle(command)

    assert events[-1].event == 'transcribe.complete'
    assert 'Hello' in (tmp_path / 'raw-transcript.md').read_text()
    assert (tmp_path / 'raw-segments.json').exists()


async def test_transcribe_run_reports_missing_audio() -> None:
    events = await handle(
        WorkerCommand(
            id='3',
            name='transcribe.run',
            payload={'audio_path': '/missing.wav', 'output_directory': '/tmp'},
        )
    )

    assert events[0].event == 'command.failed'


async def test_transcribe_run_reports_setup_when_faster_whisper_is_missing(
    mocker: MockerFixture,
    tmp_path: Path,
) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')

    mocker.patch('app.models.faster_whisper_model_factory', None)

    events = await handle(
        WorkerCommand(
            id='4',
            name='transcribe.run',
            payload={
                'audio_path': str(audio_path),
                'output_directory': str(tmp_path),
            },
        )
    )

    assert events[-1].event == 'transcribe.needs_setup'
    assert events[-1].payload['dependency'] == 'faster-whisper'


def test_faster_whisper_adapter_returns_segments_from_model() -> None:
    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            self.model_name = model_name
            self.kwargs = kwargs

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            return (
                [SimpleNamespace(start=0.0, end=1.5, text='Hello')],
                SimpleNamespace(language='en'),
            )

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='small.en',
        language='en',
        compute_type='int8',
        model_storage_directory=Path('/tmp/models'),
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.segments[0].text == 'Hello'
    assert result.language == 'en'


def test_model_installed_detects_expected_storage_names(tmp_path: Path) -> None:
    (tmp_path / 'faster-whisper-medium.en').mkdir()

    assert model_installed('medium.en', tmp_path)


async def test_models_install_reports_setup_when_faster_whisper_is_missing(mocker: MockerFixture) -> None:
    mocker.patch('app.models.faster_whisper_model_factory', None)

    events = await handle(WorkerCommand(id='5', name='models.install', payload={'model': 'small.en'}))

    assert events[0].event == 'models.install.needs_setup'
    assert events[0].payload['dependency'] == 'faster-whisper'


def test_model_install_preloads_faster_whisper_model(tmp_path: Path) -> None:
    calls: list[tuple[str, dict[str, Any]]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            calls.append((model_name, kwargs))

    result = install_faster_whisper_model(
        model_name='small.en',
        compute_type='int8',
        model_storage_directory=tmp_path,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.payload['model'] == 'small.en'
    assert calls == [('small.en', {'compute_type': 'int8', 'download_root': str(tmp_path)})]


def test_diarized_transcript_rendering_uses_turns_and_segments() -> None:
    transcript = render_diarized_transcript(
        segments=[
            {'start': 0, 'end': 2, 'text': 'Hello'},
            {'start': 2, 'end': 4, 'text': 'there'},
        ],
        turns=[{'speaker': 'Speaker 1', 'start': 0, 'end': 4}],
    )

    assert '## Speaker 1' in transcript
    assert 'Hello there' in transcript


async def test_diarize_run_completes_exact_single_speaker(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='single-speaker',
            name='diarize.run',
            payload={
                'output_directory': str(tmp_path),
                'speaker_count_mode': 'exact',
                'exact_speakers': 1,
                'segments': [
                    {'start': 1, 'end': 3, 'text': 'Hello'},
                    {'start': 3, 'end': 8, 'text': 'there'},
                ],
            },
        )
    )

    assert events[-1].event == 'diarize.complete'
    assert 'Speaker 1' in (tmp_path / 'diarized-transcript.md').read_text()
    assert (tmp_path / 'diarization.json').exists()


async def test_diarize_run_reports_backend_specific_setup(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='pyannote-setup',
            name='diarize.run',
            payload={
                'output_directory': str(tmp_path),
                'backend': 'pyannote',
                'segments': [{'start': 0, 'end': 2, 'text': 'Hello'}],
            },
        )
    )

    assert events[-1].event == 'diarize.needs_setup'
    assert events[-1].payload['dependency'] == 'pyannote-audio'


def test_summary_prompt_assembly_keeps_prompt_and_transcript() -> None:
    prompt = assemble_summary_prompt('We shipped.', 'List decisions.')

    assert prompt == 'List decisions.\n\nTranscript:\nWe shipped.'


async def test_summarize_reports_setup_when_provider_details_are_missing(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='6',
            name='summarize.run',
            payload={
                'output_directory': str(tmp_path),
                'transcript': 'We shipped.',
            },
        )
    )

    assert events[0].event == 'summarize.needs_setup'
    assert SummarizePayload.model_fields['provider_base_url'].alias in events[0].payload['missing']


async def test_openai_compatible_summary_uses_pydantic_ai_structured_output() -> None:
    agent = Agent(
        TestModel(custom_output_args={'title': 'Launch', 'summary': 'Decision made'}),
        output_type=SummaryOutput,
    )

    result = await run_openai_compatible_summary(
        provider_base_url='https://provider.test/v1',
        api_key='secret',
        model='meeting-model',
        transcript='We shipped.',
        summary_prompt='Summarize decisions.',
        title_prompt='Name the meeting.',
        agent=agent,
    )

    assert result.status == 'complete'
    assert result.title == 'Launch'
    assert result.summary == 'Decision made'


async def test_summarize_writes_provider_summary(mocker: MockerFixture, tmp_path: Path) -> None:
    agent = Agent(
        TestModel(custom_output_args={'title': 'Launch', 'summary': 'Meeting notes'}),
        output_type=SummaryOutput,
    )

    mocker.patch('app.summaries.build_summary_agent', return_value=agent)

    command = WorkerCommand(
        id='7',
        name='summarize.run',
        payload={
            'output_directory': str(tmp_path),
            'provider_base_url': 'https://provider.test/v1',
            'api_key': 'secret',
            'model': 'meeting-model',
            'transcript': 'We shipped.',
            'summary_prompt': 'Summarize decisions.',
        },
    )

    events = await handle(command)

    assert events[-1].event == 'summarize.complete'
    assert 'Meeting notes' in (tmp_path / 'summary.md').read_text()


def test_build_summary_agent_uses_openai_compatible_provider() -> None:
    agent = build_summary_agent(
        provider_base_url='https://provider.test/v1',
        api_key='secret',
        model='meeting-model',
    )

    assert isinstance(agent, Agent)


def test_raw_transcript_formatting_includes_timestamps() -> None:
    transcript = render_raw_transcript([{'start': 61, 'end': 62, 'text': 'Done'}])

    assert '[01:01 - 01:02] Done' in transcript
