import sys
from typing import Any

from app.core.pydantic_base import AppBaseModel
from app.protocol import WorkerCommand, WorkerEvent


def emit(event: WorkerEvent) -> None:
    sys.stdout.write(f'{event.model_dump_json(by_alias=True)}\n')
    sys.stdout.flush()


def command_event(
    command: WorkerCommand,
    name: str,
    payload: AppBaseModel | dict[str, Any] | None = None,
) -> WorkerEvent:
    if isinstance(payload, AppBaseModel):
        return WorkerEvent(command_id=command.id, event=name, payload=payload.model_dump(by_alias=True))

    return WorkerEvent(command_id=command.id, event=name, payload=payload or {})
