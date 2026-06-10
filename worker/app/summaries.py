from typing import cast

from pydantic_ai import Agent
from pydantic_ai.models.openai import OpenAIChatModel, OpenAIModelName
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


def assemble_summary_prompt(transcript: str, prompt: str) -> str:
    return f'{prompt.strip()}\n\nTranscript:\n{transcript.strip()}'


def summary_transcript(payload: SummarizePayload) -> str:
    if payload.transcript:
        return payload.transcript

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
    title_prompt: str,
    agent: Agent[None, SummaryOutput] | None = None,
) -> SummaryResult:
    missing = missing_summary_inputs(
        provider_base_url=provider_base_url,
        api_key=api_key,
        model=model,
        transcript=transcript,
        summary_prompt=summary_prompt,
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
                prompt=summary_response_prompt(summary_prompt=summary_prompt, title_prompt=title_prompt),
            )
        )

        return SummaryCompleteResult(title=result.output.title.strip(), summary=result.output.summary.strip())
    except Exception as error:
        return FailedResult(payload={'error': str(error)})


def build_summary_agent(provider_base_url: str, api_key: str, model: str) -> Agent[None, SummaryOutput]:
    provider = OpenAIProvider(base_url=provider_base_url, api_key=api_key)
    openai_model = OpenAIChatModel(model_name=cast(OpenAIModelName, model), provider=provider)

    return Agent(
        openai_model,
        output_type=SummaryOutput,
        system_prompt='You produce concise meeting notes.',
    )


def missing_summary_inputs(
    provider_base_url: str,
    api_key: str,
    model: str,
    transcript: str,
    summary_prompt: str,
) -> list[str]:
    missing: list[str] = []

    for name, value in [
        ('provider_base_url', provider_base_url),
        ('api_key', api_key),
        ('model', model),
        ('transcript', transcript),
        ('summary_prompt', summary_prompt),
    ]:
        if not value.strip():
            missing.append(name)

    return missing


def missing_input_aliases(missing: list[str]) -> list[str]:
    aliases: list[str] = []

    for field_name in missing:
        field = SummarizePayload.model_fields[field_name]
        aliases.append(field.alias or field_name)

    return aliases


def summary_response_prompt(summary_prompt: str, title_prompt: str) -> str:
    title_instruction = title_prompt.strip() or 'Create a concise title.'

    return f'{summary_prompt.strip()}\n\n{title_instruction}'
