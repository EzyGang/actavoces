from pathlib import Path
from types import SimpleNamespace

from pytest_mock import MockerFixture

from app.main import (
    assemble_summary_prompt,
    chat_completions_url,
    handle,
    install_faster_whisper_model,
    model_installed,
    render_diarized_transcript,
    render_raw_transcript,
    run_faster_whisper,
    run_openai_compatible_summary,
)
from app.protocol import WorkerCommand


def test_health_check_returns_ok_event() -> None:
    events = handle(WorkerCommand(id='1', name='health.check'))

    assert events[0].event == 'health.ok'


def test_transcribe_run_writes_supplied_segments(tmp_path: Path) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')
    command = WorkerCommand(
        id='2',
        name='transcribe.run',
        payload={
            'audioPath': str(audio_path),
            'outputDirectory': str(tmp_path),
            'segments': [{'start': 0, 'end': 3, 'text': ' Hello '}],
        },
    )

    events = handle(command)

    assert events[-1].event == 'transcribe.complete'
    assert 'Hello' in (tmp_path / 'raw-transcript.md').read_text()
    assert (tmp_path / 'raw-segments.json').exists()


def test_transcribe_run_reports_missing_audio() -> None:
    events = handle(
        WorkerCommand(
            id='3',
            name='transcribe.run',
            payload={'audioPath': '/missing.wav', 'outputDirectory': '/tmp'},
        )
    )

    assert events[0].event == 'command.failed'


def test_transcribe_run_reports_setup_when_faster_whisper_is_missing(mocker: MockerFixture, tmp_path: Path) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')

    def missing_model_class() -> object:
        raise ImportError

    mocker.patch('app.main.load_faster_whisper_model_class', side_effect=missing_model_class)

    events = handle(
        WorkerCommand(
            id='4',
            name='transcribe.run',
            payload={
                'audioPath': str(audio_path),
                'outputDirectory': str(tmp_path),
            },
        )
    )

    assert events[-1].event == 'transcribe.needs_setup'
    assert events[-1].payload['dependency'] == 'faster-whisper'


def test_faster_whisper_adapter_returns_segments_from_model() -> None:
    class FakeModel:
        def __init__(self, model_name: str, **kwargs: object) -> None:
            self.model_name = model_name
            self.kwargs = kwargs

        def transcribe(self, audio_path: str, **kwargs: object) -> object:
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

    assert result['status'] == 'complete'
    assert result['segments'][0]['text'] == 'Hello'
    assert result['language'] == 'en'


def test_model_installed_detects_expected_storage_names(tmp_path: Path) -> None:
    (tmp_path / 'faster-whisper-medium.en').mkdir()

    assert model_installed('medium.en', tmp_path)


def test_models_install_reports_setup_when_faster_whisper_is_missing(mocker: MockerFixture) -> None:
    def missing_model_class() -> object:
        raise ImportError

    mocker.patch('app.main.load_faster_whisper_model_class', side_effect=missing_model_class)

    events = handle(WorkerCommand(id='5', name='models.install', payload={'model': 'small.en'}))

    assert events[0].event == 'models.install.needs_setup'
    assert events[0].payload['dependency'] == 'faster-whisper'


def test_model_install_preloads_faster_whisper_model(tmp_path: Path) -> None:
    calls: list[tuple[str, dict[str, object]]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: object) -> None:
            calls.append((model_name, kwargs))

    result = install_faster_whisper_model(
        model_name='small.en',
        compute_type='int8',
        model_storage_directory=tmp_path,
        model_factory=FakeModel,
    )

    assert result['status'] == 'complete'
    assert result['payload']['model'] == 'small.en'
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


def test_diarize_run_completes_exact_single_speaker(tmp_path: Path) -> None:
    events = handle(
        WorkerCommand(
            id='single-speaker',
            name='diarize.run',
            payload={
                'outputDirectory': str(tmp_path),
                'speakerCountMode': 'exact',
                'exactSpeakers': 1,
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


def test_diarize_run_reports_backend_specific_setup(tmp_path: Path) -> None:
    events = handle(
        WorkerCommand(
            id='pyannote-setup',
            name='diarize.run',
            payload={
                'outputDirectory': str(tmp_path),
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


def test_summarize_reports_setup_when_provider_details_are_missing(tmp_path: Path) -> None:
    events = handle(
        WorkerCommand(
            id='6',
            name='summarize.run',
            payload={
                'outputDirectory': str(tmp_path),
                'transcript': 'We shipped.',
            },
        )
    )

    assert events[0].event == 'summarize.needs_setup'
    assert 'providerBaseUrl' in events[0].payload['missing']


def test_openai_compatible_summary_posts_chat_completion() -> None:
    requests: list[object] = []

    class FakeResponse:
        def __enter__(self) -> FakeResponse:
            return self

        def __exit__(self, *_args: object) -> None:
            return None

        def read(self) -> bytes:
            return (
                b'{"choices":[{"message":{"content":"{\\"title\\":\\"Launch\\",\\"summary\\":\\"Decision made\\"}"}}]}'
            )

    def fake_urlopen(http_request: object, timeout: int) -> FakeResponse:
        requests.append(http_request)
        assert timeout == 60

        return FakeResponse()

    result = run_openai_compatible_summary(
        provider_base_url='https://provider.test/v1',
        api_key='secret',
        model='meeting-model',
        transcript='We shipped.',
        summary_prompt='Summarize decisions.',
        title_prompt='Name the meeting.',
        urlopen=fake_urlopen,
    )

    assert result == {'status': 'complete', 'title': 'Launch', 'summary': 'Decision made'}
    assert len(requests) == 1
    assert requests[0].full_url == 'https://provider.test/v1/chat/completions'
    assert requests[0].headers['Authorization'] == 'Bearer secret'


def test_summarize_writes_provider_summary(mocker: MockerFixture, tmp_path: Path) -> None:
    def fake_summary(**_kwargs: object) -> dict[str, object]:
        return {'status': 'complete', 'title': 'Launch', 'summary': 'Meeting notes'}

    mocker.patch('app.main.run_openai_compatible_summary', side_effect=fake_summary)

    command = WorkerCommand(
        id='7',
        name='summarize.run',
        payload={
            'outputDirectory': str(tmp_path),
            'providerBaseUrl': 'https://provider.test/v1',
            'apiKey': 'secret',
            'model': 'meeting-model',
            'transcript': 'We shipped.',
            'summaryPrompt': 'Summarize decisions.',
        },
    )

    events = handle(command)

    assert events[-1].event == 'summarize.complete'
    assert 'Meeting notes' in (tmp_path / 'summary.md').read_text()


def test_chat_completions_url_accepts_full_endpoint() -> None:
    assert (
        chat_completions_url('https://provider.test/v1/chat/completions') == 'https://provider.test/v1/chat/completions'
    )


def test_raw_transcript_formatting_includes_timestamps() -> None:
    transcript = render_raw_transcript([{'start': 61, 'end': 62, 'text': 'Done'}])

    assert '[01:01 - 01:02] Done' in transcript
