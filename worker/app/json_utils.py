from pathlib import Path
from typing import Any

import orjson


def loads(value: str | bytes) -> Any:
    return orjson.loads(value)


def dumps(value: Any) -> str:
    return orjson.dumps(value, option=orjson.OPT_INDENT_2).decode('utf-8')


def write_json(path: Path, value: Any) -> None:
    path.write_text(f'{dumps(value)}\n', encoding='utf-8')


def write_text(path: Path, value: str) -> None:
    path.write_text(value, encoding='utf-8')
