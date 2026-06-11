from pydantic_ai import Agent, ModelSettings
from pydantic_ai.models.openai import OpenAIChatModel
from pydantic_ai.providers.openai import OpenAIProvider

from app.dtos import (
    FailedResult,
    NeedsSetupResult,
    SummarizePayload,
    SummaryCompleteResult,
    SummaryOutput,
    SummarySetupPayload,
)


type SummaryResult = NeedsSetupResult | FailedResult | SummaryCompleteResult

DEFAULT_SUMMARY_PROMPT = 'Summarize decisions, action items, risks, and unanswered questions.'
TITLE_INSTRUCTION = 'Create a concise meeting title from the transcript.'


def assemble_summary_prompt(transcript: str, prompt: str) -> str:
    return f'{prompt.strip()}\n\nTranscript:\n{transcript.strip()}'


def summary_transcript(payload: SummarizePayload) -> str:
    for path in (payload.diarized_transcript_path, payload.transcript_path):
        if path is not None and path.exists():
            return path.read_text(encoding='utf-8')

    return ''


async def run_openai_compatible_summary(
    provider_base_url: str,
    api_key: str,
    model: str,
    transcript: str,
    summary_prompt: str,
    agent: Agent[None, SummaryOutput] | None = None,
) -> SummaryResult:
    missing = missing_summary_inputs(
        provider_base_url=provider_base_url,
        model=model,
        transcript=transcript,
    )

    if missing:
        return NeedsSetupResult(
            payload=SummarySetupPayload(
                missing=missing_input_aliases(missing=missing),
                provider=provider_base_url or 'OpenAI-compatible',
            ).model_dump(by_alias=True),
        )

    try:
        summary_agent = agent or build_summary_agent(
            provider_base_url=provider_base_url,
            api_key=api_key,
            model=model,
        )
        result = await summary_agent.run(
            assemble_summary_prompt(
                transcript=transcript,
                prompt=summary_response_prompt(summary_prompt=summary_prompt),
            )
        )

        return SummaryCompleteResult(title=result.output.title.strip(), summary=result.output.summary.strip())
    except Exception as error:
        return FailedResult(payload={'error': str(error)})


def build_summary_agent(provider_base_url: str, api_key: str, model: str) -> Agent[None, SummaryOutput | str]:
    provider = OpenAIProvider(base_url=provider_base_url, api_key=api_key)
    openai_model = OpenAIChatModel(model_name=model, provider=provider, settings=ModelSettings(tool_choice='auto'))

    return Agent(
        openai_model,
        output_type=[SummaryOutput, str],
        retries=3,
        instructions=(
            'You are a transcript summary/title creator agent. '
            'Make sure to return result as a structured output type defined'
        ),
    )


def missing_summary_inputs(
    provider_base_url: str,
    model: str,
    transcript: str,
) -> list[str]:
    missing: list[str] = []

    for name, value in [
        ('provider_base_url', provider_base_url),
        ('model', model),
        ('transcript', transcript),
    ]:
        if not value.strip():
            missing.append(name)

    return missing


def missing_input_aliases(missing: list[str]) -> list[str]:
    return missing


def summary_response_prompt(summary_prompt: str) -> str:
    prompt = summary_prompt.strip() or DEFAULT_SUMMARY_PROMPT

    return f'{prompt}\n\n{TITLE_INSTRUCTION}'
