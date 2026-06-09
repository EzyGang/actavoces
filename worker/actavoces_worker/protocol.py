from typing import Literal

from pydantic import BaseModel, Field


WorkerCommandName = Literal[
    "health.check",
    "models.status",
    "models.install",
    "transcribe.run",
    "diarize.run",
    "summarize.run",
]


class WorkerCommand(BaseModel):
    id: str
    name: WorkerCommandName
    payload: dict = Field(default_factory=dict)


class WorkerEvent(BaseModel):
    command_id: str
    event: str
    payload: dict = Field(default_factory=dict)
