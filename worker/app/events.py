import sys
from typing import Any

from app.protocol import WorkerCommand, WorkerEvent


def emit(event: WorkerEvent) -> None:
    sys.stdout.write(f'{event.model_dump_json()}\n')
    sys.stdout.flush()


def command_event(command: WorkerCommand, name: str, payload: dict[str, Any] | None = None) -> WorkerEvent:
    return WorkerEvent(command_id=command.id, event=name, payload=payload or {})
