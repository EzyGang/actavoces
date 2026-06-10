import asyncio
import sys

from app.events import emit
from app.handlers import handle
from app.json_utils import loads
from app.protocol import WorkerCommand


def main() -> None:
    asyncio.run(run())


async def run() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue

        for worker_event in await handle(WorkerCommand.model_validate(loads(line))):
            emit(worker_event)


if __name__ == '__main__':
    main()
