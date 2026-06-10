from pydantic import BaseModel, ConfigDict


def to_camel(value: str) -> str:
    words = value.split('_')

    return f'{words[0]}{"".join(word.capitalize() for word in words[1:])}'


class AppBaseModel(BaseModel):
    model_config = ConfigDict(alias_generator=to_camel, populate_by_name=True)
