from typing import Any

from pydantic import Field

from app.core.pydantic_base import AppBaseModel


WorkerCommandName = str


class WorkerCommand(AppBaseModel):
    id: str
    name: WorkerCommandName
    payload: dict[str, Any] = Field(default_factory=dict)


class WorkerEvent(AppBaseModel):
    command_id: str
    event: str
    payload: dict[str, Any] = Field(default_factory=dict)
