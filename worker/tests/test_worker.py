from pathlib import Path
from types import SimpleNamespace
from typing import Any

from pydantic_ai import Agent
from pydantic_ai.models.test import TestModel
from pytest_mock import MockerFixture

from app.dtos import (
    Segment,
    SummarizePayload,
    SummaryOutput,
    TranscriptionMetadata,
    TranscriptionVadOptions,
    TranscriptionWord,
)
from app.events import emit
from app.formatting import render_diarized_transcript, render_raw_transcript
from app.handlers import handle
from app.json_utils import loads
from app.models import (
    cuda_status,
    install_faster_whisper_model,
    model_installed,
    normalized_transcription_context,
    run_faster_whisper,
)
from app.protocol import WorkerCommand, WorkerEvent
from app.summaries import (
    assemble_summary_prompt,
    build_summary_agent,
    run_openai_compatible_summary,
    summary_transcript,
)


class MockPyannoteTurn:
    def __init__(self, start: float, end: float) -> None:
        self.start = start
        self.end = end


class MockPyannoteAnnotation:
    def itertracks(self, yield_label: bool = False) -> list[tuple[MockPyannoteTurn, None, str]]:
        return [
            (MockPyannoteTurn(0, 1), None, 'SPEAKER_00'),
            (MockPyannoteTurn(1, 2), None, 'SPEAKER_01'),
        ]


class MockPyannotePipeline:
    def __init__(self) -> None:
        self.calls: list[dict[str, int]] = []

    def __call__(self, audio_path: str, **kwargs: int) -> MockPyannoteAnnotation:
        self.calls.append(kwargs)

        return MockPyannoteAnnotation()


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
            'title': 'Planning Call',
            'segments': [{'start': 0, 'end': 3, 'text': ' Hello '}],
        },
    )

    events = await handle(command)

    assert events[-1].event == 'transcribe.complete'
    transcript = (tmp_path / 'raw-transcript.md').read_text()
    assert '# Raw transcript - Planning Call' in transcript
    assert 'Hello' in transcript
    assert (tmp_path / 'meta' / 'raw-segments.json').exists()
    raw_words = loads((tmp_path / 'meta' / 'raw-words.json').read_text())
    assert raw_words == {'words': []}


async def test_transcribe_run_writes_transcription_metadata(tmp_path: Path) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')

    events = await handle(
        WorkerCommand(
            id='metadata',
            name='transcribe.run',
            payload={
                'audioPath': str(audio_path),
                'outputDirectory': str(tmp_path),
                'model': 'small',
                'language': 'en',
                'segments': [{'start': 2, 'end': 5, 'text': ' Metadata '}],
            },
        )
    )

    metadata = loads((tmp_path / 'meta' / 'transcription.json').read_text())
    assert events[-1].event == 'transcribe.complete'
    assert metadata['model'] == 'small'
    assert metadata['language'] == 'en'
    assert metadata['transcriptionProfile'] == 'conservative_vad'
    assert metadata['vad']['enabled'] is True
    assert metadata['vad']['profile'] == 'conservative_vad'
    assert metadata['vad']['parameters']['min_silence_duration_ms'] == 2000
    assert metadata['sourceStart'] == 2
    assert metadata['sourceEnd'] == 5


def test_transcription_metadata_serializes_aliases() -> None:
    metadata = TranscriptionMetadata(
        model='small',
        language='en',
        vad=TranscriptionVadOptions(parameters={'min_silence_duration_ms': 2000}),
        source_start=2,
        source_end=5,
    ).model_dump(by_alias=True)

    assert metadata['transcriptionProfile'] == 'conservative_vad'
    assert metadata['sourceStart'] == 2
    assert metadata['sourceEnd'] == 5
    assert metadata['vad']['profile'] == 'conservative_vad'


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
    transcribe_calls: list[dict[str, Any]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            self.model_name = model_name
            self.kwargs = kwargs

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            transcribe_calls.append(kwargs)

            return (
                [
                    SimpleNamespace(
                        start=0.0,
                        end=1.5,
                        text='Hello world',
                        words=[
                            SimpleNamespace(start=0.0, end=0.5, word='Hello', probability=0.95),
                            SimpleNamespace(start=0.6, end=1.5, word='world', probability=0.9),
                        ],
                    )
                ],
                SimpleNamespace(language='en'),
            )

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='en',
        compute_type='int8',
        model_storage_directory=Path('/tmp/models'),
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.segments[0].text == 'Hello world'
    assert result.words[0].segment_id == 0
    assert result.words[0].text == 'Hello'
    assert result.words[0].start == 0.0
    assert result.words[0].end == 0.5
    assert result.words[0].probability == 0.95
    assert result.words[1].text == 'world'
    assert result.language == 'en'
    assert transcribe_calls == [
        {
            'vad_filter': True,
            'vad_parameters': {
                'threshold': 0.5,
                'min_speech_duration_ms': 0,
                'min_silence_duration_ms': 2000,
                'speech_pad_ms': 400,
            },
            'word_timestamps': True,
            'language': 'en',
        }
    ]


def test_faster_whisper_adapter_passes_transcription_context_prompt() -> None:
    transcribe_calls: list[dict[str, Any]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            pass

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            transcribe_calls.append(kwargs)

            return ([SimpleNamespace(start=0.0, end=1.0, text='ActaVoces')], SimpleNamespace(language='en'))

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='auto',
        transcription_context=' ActaVoces \n\nKaneo\nActaVoces ',
        compute_type='int8',
        model_storage_directory=None,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert transcribe_calls[0]['initial_prompt'] == 'ActaVoces\nKaneo'
    assert 'language' not in transcribe_calls[0]


def test_blank_transcription_context_does_not_pass_initial_prompt() -> None:
    transcribe_calls: list[dict[str, Any]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            pass

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            transcribe_calls.append(kwargs)

            return ([SimpleNamespace(start=0.0, end=1.0, text='Hello')], SimpleNamespace(language='en'))

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='en',
        transcription_context='\n  \n',
        compute_type='int8',
        model_storage_directory=None,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert 'initial_prompt' not in transcribe_calls[0]
    assert transcribe_calls[0]['language'] == 'en'


def test_transcription_context_normalization_bounds_prompt() -> None:
    context = normalized_transcription_context(context=f'ActaVoces\n\nKaneo\nActaVoces\n{"a" * 4100}')

    assert context.startswith('ActaVoces\nKaneo\n')
    assert len(context) == 4000


def test_faster_whisper_adapter_handles_missing_words() -> None:
    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            pass

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            return ([SimpleNamespace(start=0.0, end=1.0, text='No words')], SimpleNamespace(language='en'))

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='en',
        compute_type='int8',
        model_storage_directory=None,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.segments[0].text == 'No words'
    assert result.words == []


async def test_transcribe_run_writes_raw_words_from_faster_whisper(mocker: MockerFixture, tmp_path: Path) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')
    result = SimpleNamespace(
        status='complete',
        segments=[Segment(id=0, start=0, end=1, text='Hello')],
        words=[TranscriptionWord(segment_id=0, text='Hello', start=0, end=1, probability=0.95)],
        language='en',
        warning=None,
    )
    mocker.patch('app.handlers.run_faster_whisper', return_value=result)

    events = await handle(
        WorkerCommand(
            id='words',
            name='transcribe.run',
            payload={'audio_path': str(audio_path), 'output_directory': str(tmp_path)},
        )
    )

    raw_words = loads((tmp_path / 'meta' / 'raw-words.json').read_text())
    raw_segments = loads((tmp_path / 'meta' / 'raw-segments.json').read_text())

    assert events[-1].event == 'transcribe.complete'
    assert events[-1].payload['wordsPath'] == str(tmp_path / 'meta' / 'raw-words.json')
    assert raw_words == {'words': [{'segment_id': 0, 'text': 'Hello', 'start': 0, 'end': 1, 'probability': 0.95}]}
    assert raw_segments == {'segments': [{'id': 0, 'start': 0, 'end': 1, 'text': 'Hello'}]}


def test_faster_whisper_cuda_fallback_uses_cpu_when_cuda_libraries_are_missing() -> None:
    calls: list[dict[str, Any]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            calls.append(kwargs)
            if kwargs.get('device') == 'cuda':
                raise RuntimeError('Library cublas64_12.dll is not found or cannot be loaded')

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            return (
                [SimpleNamespace(start=0.0, end=1.0, text='CPU fallback')],
                SimpleNamespace(language='en'),
            )

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='en',
        compute_type='cuda',
        model_storage_directory=None,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.warning == 'CUDA libraries are unavailable; CPU fallback was used.'
    assert calls == [
        {'device': 'cuda', 'compute_type': 'int8_float16'},
        {'device': 'cpu', 'compute_type': 'int8'},
    ]


def test_faster_whisper_auto_fallback_uses_cpu_when_cuda_libraries_are_missing() -> None:
    calls: list[dict[str, Any]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            calls.append(kwargs)
            if not kwargs:
                raise RuntimeError('Library cublas64_12.dll is not found or cannot be loaded')

        def transcribe(self, audio_path: str, **kwargs: Any) -> tuple[list[SimpleNamespace], SimpleNamespace]:
            return (
                [SimpleNamespace(start=0.0, end=1.0, text='CPU fallback')],
                SimpleNamespace(language='en'),
            )

    result = run_faster_whisper(
        audio_path=Path('recording.wav'),
        model_name='medium',
        language='en',
        compute_type='auto',
        model_storage_directory=None,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.warning == 'CUDA libraries are unavailable; CPU fallback was used.'
    assert calls == [{}, {'device': 'cpu', 'compute_type': 'int8'}]


async def test_runtime_capabilities_reports_cuda_status(mocker: MockerFixture) -> None:
    mocker.patch('app.handlers.cuda_status', return_value=(False, 'missing cuDNN'))

    events = await handle(WorkerCommand(id='capabilities', name='runtime.capabilities'))

    assert events[0].event == 'runtime.capabilities'
    assert events[0].payload['fasterWhisperAvailable'] is True
    assert events[0].payload['cudaAvailable'] is False
    assert events[0].payload['cudaError'] == 'missing cuDNN'


def test_cuda_status_requires_nvidia_libraries(mocker: MockerFixture) -> None:
    mocker.patch('app.models.faster_whisper_model_factory', SimpleNamespace())
    mocker.patch('app.models.sys.platform', 'win32')
    mocker.patch('app.models.ctranslate2_module.get_cuda_device_count', return_value=1)
    mocker.patch('app.models.ctranslate2_module.get_supported_compute_types', return_value={'int8_float16'})

    def load_library(name: str) -> None:
        if name == 'cudnn64_9.dll':
            raise OSError('missing')

    mocker.patch('app.models.ctypes.CDLL', side_effect=load_library)

    assert cuda_status() == (False, 'Missing NVIDIA libraries: cudnn64_9.dll')


def test_model_installed_detects_expected_storage_names(tmp_path: Path) -> None:
    (tmp_path / 'faster-whisper-medium').mkdir()

    assert model_installed('medium', tmp_path)


async def test_models_install_reports_setup_when_faster_whisper_is_missing(mocker: MockerFixture) -> None:
    mocker.patch('app.models.faster_whisper_model_factory', None)

    events = await handle(WorkerCommand(id='5', name='models.install', payload={'model': 'medium'}))

    assert events[0].event == 'models.install.needs_setup'
    assert events[0].payload['dependency'] == 'faster-whisper'


def test_model_install_preloads_faster_whisper_model(tmp_path: Path) -> None:
    calls: list[tuple[str, dict[str, Any]]] = []

    class FakeModel:
        def __init__(self, model_name: str, **kwargs: Any) -> None:
            calls.append((model_name, kwargs))

    result = install_faster_whisper_model(
        model_name='medium',
        compute_type='int8',
        model_storage_directory=tmp_path,
        model_factory=FakeModel,
    )

    assert result.status == 'complete'
    assert result.payload['model'] == 'medium'
    assert calls == [('medium', {'compute_type': 'int8', 'download_root': str(tmp_path)})]


def test_diarized_transcript_rendering_uses_turns_and_segments() -> None:
    transcript = render_diarized_transcript(
        segments=[
            {'start': 0, 'end': 2, 'text': 'Hello'},
            {'start': 2, 'end': 4, 'text': 'there'},
        ],
        turns=[{'speaker': 'Speaker 1', 'start': 0, 'end': 4}],
        title='Planning Call',
    )

    assert '# Diarized transcript - Planning Call' in transcript
    assert '## Speaker 1' in transcript
    assert 'Hello there' in transcript


def test_diarized_transcript_splits_mixed_speaker_words_inside_segment() -> None:
    transcript = render_diarized_transcript(
        segments=[{'start': 0, 'end': 4, 'text': 'Hello yes continue'}],
        words=[
            TranscriptionWord(segment_id=0, text='Hello', start=0, end=0.5),
            TranscriptionWord(segment_id=0, text='yes', start=1.0, end=1.2),
            TranscriptionWord(segment_id=0, text='continue', start=1.4, end=2.0),
        ],
        turns=[
            {'speaker': 'Speaker 1', 'start': 0, 'end': 0.8},
            {'speaker': 'Speaker 2', 'start': 0.9, 'end': 1.3},
            {'speaker': 'Speaker 1', 'start': 1.3, 'end': 3},
        ],
    )

    assert '[00:00 - 00:00] Hello' in transcript
    assert '[00:01 - 00:01] yes' in transcript
    assert '[00:01 - 00:02] continue' in transcript


def test_diarized_transcript_keeps_short_backchannel_separate() -> None:
    transcript = render_diarized_transcript(
        segments=[{'start': 0, 'end': 4, 'text': 'I think yes we should ship'}],
        words=[
            TranscriptionWord(segment_id=0, text='I', start=0, end=0.2),
            TranscriptionWord(segment_id=0, text='think', start=0.3, end=0.6),
            TranscriptionWord(segment_id=0, text='yes', start=0.7, end=0.9),
            TranscriptionWord(segment_id=0, text='we', start=1.0, end=1.2),
            TranscriptionWord(segment_id=0, text='should', start=1.3, end=1.6),
            TranscriptionWord(segment_id=0, text='ship', start=1.7, end=2.0),
        ],
        turns=[
            {'speaker': 'Speaker 1', 'start': 0, 'end': 2.5},
            {'speaker': 'Speaker 2', 'start': 0.65, 'end': 0.95},
        ],
    )

    assert '[00:00 - 00:00] I think' in transcript
    assert '[00:00 - 00:00] yes' in transcript
    assert '[00:01 - 00:02] we should ship' in transcript


def test_diarized_transcript_uses_nearest_turn_for_no_overlap_words() -> None:
    transcript = render_diarized_transcript(
        segments=[{'start': 5, 'end': 6, 'text': 'between'}],
        words=[TranscriptionWord(segment_id=0, text='between', start=5, end=6)],
        turns=[
            {'speaker': 'Speaker 1', 'start': 0, 'end': 1},
            {'speaker': 'Speaker 2', 'start': 7, 'end': 8},
        ],
    )

    assert '## Speaker 2' in transcript
    assert '[00:05 - 00:06] between' in transcript


async def test_diarize_run_completes_exact_single_speaker(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='single-speaker',
            name='diarize.run',
            payload={
                'output_directory': str(tmp_path),
                'speaker_count_mode': 'exact',
                'exact_speakers': 1,
                'title': 'Planning Call',
                'segments': [
                    {'start': 1, 'end': 3, 'text': 'Hello'},
                    {'start': 3, 'end': 8, 'text': 'there'},
                ],
            },
        )
    )

    assert events[-1].event == 'diarize.complete'
    transcript = (tmp_path / 'diarized-transcript.md').read_text()
    assert '# Diarized transcript - Planning Call' in transcript
    assert 'Speaker 1' in transcript
    assert (tmp_path / 'meta' / 'diarization.json').exists()


async def test_diarize_run_writes_speaker_labeled_artifacts(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='speaker-words',
            name='diarize.run',
            payload={
                'outputDirectory': str(tmp_path),
                'speakerCountMode': 'exact',
                'exactSpeakers': 1,
                'segments': [{'start': 0, 'end': 1, 'text': 'Hello'}],
                'words': [{'segmentId': 0, 'text': 'Hello', 'start': 0, 'end': 1, 'probability': 0.95}],
            },
        )
    )

    words = loads((tmp_path / 'meta' / 'speaker-labeled-words.json').read_text())
    utterances = loads((tmp_path / 'meta' / 'speaker-labeled-utterances.json').read_text())

    assert events[-1].event == 'diarize.complete'
    assert words['words'][0]['speaker'] == 'Speaker 1'
    assert utterances['utterances'][0]['text'] == 'Hello'


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
    assert events[-1].payload['dependency'] == 'pyannote.audio'


async def test_diarization_check_reports_missing_token(mocker: MockerFixture) -> None:
    mocker.patch('app.diarization.pyannote_pipeline_factory', SimpleNamespace())
    mocker.patch('app.diarization.shutil.which', return_value='ffmpeg')

    events = await handle(WorkerCommand(id='pyannote-check', name='diarization.check'))

    assert events[-1].event == 'diarization.needs_setup'
    assert events[-1].payload['dependency'] == 'hugging-face-token'


async def test_pyannote_diarize_run_writes_normalized_speakers(tmp_path: Path, mocker: MockerFixture) -> None:
    audio_path = tmp_path / 'recording.wav'
    audio_path.write_bytes(b'RIFFdata')
    pipeline = MockPyannotePipeline()
    factory = SimpleNamespace(from_pretrained=lambda checkpoint, token: pipeline)
    mocker.patch('app.diarization.pyannote_pipeline_factory', factory)
    mocker.patch('app.diarization.shutil.which', return_value='ffmpeg')

    events = await handle(
        WorkerCommand(
            id='pyannote-run',
            name='diarize.run',
            payload={
                'audioPath': str(audio_path),
                'outputDirectory': str(tmp_path),
                'backend': 'pyannote',
                'apiKey': 'hf_secret',
                'speakerCountMode': 'range',
                'minSpeakers': 1,
                'maxSpeakers': 2,
                'segments': [
                    {'start': 0, 'end': 1, 'text': 'Hello'},
                    {'start': 1, 'end': 2, 'text': 'Hi'},
                ],
            },
        )
    )

    assert events[-1].event == 'diarize.complete'
    assert 'Speaker 1' in (tmp_path / 'diarized-transcript.md').read_text()
    assert 'Speaker 2' in (tmp_path / 'diarized-transcript.md').read_text()
    assert pipeline.calls == [{'min_speakers': 1, 'max_speakers': 2}]


def test_summary_prompt_assembly_keeps_prompt_and_transcript() -> None:
    prompt = assemble_summary_prompt('We shipped.', 'List decisions.')

    assert 'List decisions.' in prompt
    assert 'Transcript:' in prompt
    assert 'We shipped.' in prompt


def test_summary_transcript_prefers_diarized_transcript(tmp_path: Path) -> None:
    diarized_path = tmp_path / 'diarized-transcript.md'
    raw_path = tmp_path / 'raw-transcript.md'
    diarized_path.write_text('Speaker 1: Diarized notes')
    raw_path.write_text('Raw notes')

    transcript = summary_transcript(
        SummarizePayload(
            output_directory=tmp_path,
            diarized_transcript_path=diarized_path,
            transcript_path=raw_path,
        )
    )

    assert transcript == 'Speaker 1: Diarized notes'


def test_summary_transcript_falls_back_when_diarized_transcript_is_empty(tmp_path: Path) -> None:
    diarized_path = tmp_path / 'diarized-transcript.md'
    raw_path = tmp_path / 'raw-transcript.md'
    diarized_path.write_text('')
    raw_path.write_text('Raw notes')

    transcript = summary_transcript(
        SummarizePayload(
            output_directory=tmp_path,
            diarized_transcript_path=diarized_path,
            transcript_path=raw_path,
        )
    )

    assert transcript == 'Raw notes'


async def test_summarize_reports_setup_when_provider_details_are_missing(tmp_path: Path) -> None:
    events = await handle(
        WorkerCommand(
            id='6',
            name='summarize.run',
            payload={
                'output_directory': str(tmp_path),
            },
        )
    )

    assert events[0].event == 'summarize.needs_setup'
    assert 'provider_base_url' in events[0].payload['missing']


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
            'model': 'meeting-model',
            'summary_prompt': 'Summarize decisions.',
            'transcript_path': str(tmp_path / 'raw-transcript.md'),
        },
    )
    (tmp_path / 'raw-transcript.md').write_text('We shipped.')

    events = await handle(command)

    assert events[-1].event == 'summarize.complete'
    assert 'Meeting notes' in (tmp_path / 'meta' / 'summary.md').read_text()


def test_build_summary_agent_uses_openai_compatible_provider() -> None:
    agent = build_summary_agent(
        provider_base_url='https://provider.test/v1',
        api_key='secret',
        model='meeting-model',
    )

    assert isinstance(agent, Agent)


def test_raw_transcript_formatting_includes_timestamps() -> None:
    transcript = render_raw_transcript([{'start': 61, 'end': 62, 'text': 'Done'}], title='Retro')

    assert '# Raw transcript - Retro' in transcript
    assert '[01:01 - 01:02] Done' in transcript
