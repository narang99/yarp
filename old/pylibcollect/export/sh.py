

from pathlib import Path
from typing import Union


def quote(s: Union[Path, str], quote_char: str = "'") -> str:
    return f"""{quote_char}{s}{quote_char}"""