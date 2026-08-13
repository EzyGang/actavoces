import asyncio
import sys
from typing import Any, cast

from app.events import emit
from app.handlers import handle
from app.json_utils import loads
from app.protocol import WorkerCommand, WorkerEvent


def main() -> None:
    asyncio.run(run())


async def run() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue

        for worker_event in await handle_line(line=line):
            emit(worker_event)


async def handle_line(line: str) -> list[WorkerEvent]:
    command_id = 'unknown'

    try:
        data = loads(line)
        command_id = command_id_from(data=data)

        return await handle(WorkerCommand.model_validate(data))
    except Exception as error:
        return [WorkerEvent(command_id=command_id, event='command.failed', payload={'error': str(error)})]


def command_id_from(data: Any) -> str:
    if not isinstance(data, dict):
        return 'unknown'

    command = cast(dict[str, Any], data)
    command_id = command.get('id')

    return command_id if isinstance(command_id, str) else 'unknown'


if __name__ == '__main__':
    main()
