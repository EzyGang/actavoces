---
name: good-python
description: Python code quality rules for writing, editing, or reviewing Python 3.14 code. Use whenever working on Python implementation, refactors, tests, typing, Pydantic models, JSON handling, or lint/type-check fixes.
---

# Good Python

Guidelines for writing clean, typed, maintainable Python 3.14 code.

## Core Principles

- Prefer the smallest clear implementation that solves the problem.
- Keep code simple, readable, and direct.
- Follow DRY strictly. Do not repeat code unless there is truly no practical alternative.
- Split functions longer than 30 lines unless there is a rare, strong reason not to.
- Split files longer than 200 lines, except files defining database models.
- Do not create comments unless absolutely necessary.
- Do not add docstrings or explanatory documentation unless the code is critical and confusing, or the user explicitly asks for it.

## Typing

- All function arguments and return values must be type annotated.
- Annotate generic containers explicitly, such as `list[str]`, `dict[str, Any]`, and `tuple[int, str]`.
- Do not use `object` in annotations. Use `Any` or a more specific type.
- Do not silence type checkers. If an existing suppression is already present, leave it alone unless the task is to fix it.
- If a type issue cannot be fixed confidently, highlight it instead of suppressing it.
- Use Python 3.14 generic parameter syntax for generic functions and classes, such as `def foo[**P, T](...) -> T:`.

## Function Calls

- Prefer keyword arguments over positional arguments: `func(a=1, b=2, c=3)` is preferred over `func(1, 2, 3)`.
- Positional arguments are acceptable for clear built-ins, tiny local helpers, and APIs where keywords are unsupported or reduce readability.

## Pydantic

- When creating Pydantic models from existing data with matching field names, always use `.model_validate()`, `.model_validate_json()`, or `.model_validate(obj, from_attributes=True)`.
- This applies to dictionaries, JSON strings/bytes, ORM objects, nested models, and lists of models.
- Do not manually unpack matching dictionaries into Pydantic constructors.

```python
user = User.model_validate(data)
users = [User.model_validate(item) for item in items]
payload = Payload.model_validate_json(raw_json)
record = Record.model_validate(row, from_attributes=True)
```

## Imports

- Do not use local imports unless necessary to avoid circular imports.
- Do not re-export from `__init__.py`; import directly from the module that defines the symbol.
- Do not define `__all__`.
- Use `orjson` instead of the built-in `json` module.

## Return Types

- Do not return plain `dict` or `list[dict]` from functions for complex data.
- Use Pydantic `BaseModel` DTOs for structured return values.
- Simple internal mappings are acceptable when a mapping is genuinely the domain shape and is fully typed.

## Strings

- Use f-strings for interpolation.
- Do not use `.format()` or `%` formatting for new code.

```python
message = f'user {user_id} not found'
```

## Formatting

- Line length is 120 characters.
- Use single quotes.
- Use spaces for indentation.
- Target Python 3.14.
