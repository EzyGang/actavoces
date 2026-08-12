import asyncio
import sys

from app.events import emit
from app.handlers import handle
from app.protocol import WorkerCommand, WorkerEvent


def main() -> None:
    asyncio.run(run())


async def run() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue

        command = WorkerCommand.model_validate_json(line)
        for worker_event in await handle(command):
            emit(worker_event)
        emit(WorkerEvent(command_id=command.id, event='command.finished'))


if __name__ == '__main__':
    main()
