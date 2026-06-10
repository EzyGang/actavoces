from typing import Any, Literal

from pydantic import Field

from app.core.pydantic_base import AppBaseModel


WorkerCommandName = Literal[
    'health.check',
    'runtime.capabilities',
    'models.status',
    'models.install',
    'diarization.check',
    'transcribe.run',
    'diarize.run',
    'summarize.run',
]


class WorkerCommand(AppBaseModel):
    id: str
    name: WorkerCommandName
    payload: dict[str, Any] = Field(default_factory=dict)


class WorkerEvent(AppBaseModel):
    command_id: str
    event: str
    payload: dict[str, Any] = Field(default_factory=dict)
