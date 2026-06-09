import json
import sys

from actavoces_worker.protocol import WorkerCommand, WorkerEvent


def emit(event: WorkerEvent) -> None:
    sys.stdout.write(f"{event.model_dump_json()}\n")
    sys.stdout.flush()


def handle(command: WorkerCommand) -> None:
    if command.name == "health.check":
        emit(
            WorkerEvent(
                command_id=command.id,
                event="health.ok",
                payload={"worker": "actavoces-worker"},
            )
        )
        return

    emit(
        WorkerEvent(
            command_id=command.id,
            event="command.unsupported",
            payload={"name": command.name},
        )
    )


def main() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue

        handle(WorkerCommand.model_validate(json.loads(line)))


if __name__ == "__main__":
    main()
