"""module to fake the library search that individual FFI libraries do for loading dynamic libraries

See docs/SEARCH.md for details on search algorithms used


Thankfully for us, ctypes defines all the searching we need to do amazingly well
Both cffi and ctypes use this (ctypes.util.find_library has all the semantics defined in dyld man page)
"""

import ctypes.util
from pathlib import Path
from typing import Optional


def fallback_find_library(name: str) -> Optional[str]:
    if Path(name).exists():
        return name
    return ctypes.util.find_library(name)


def cffi_find_library(name: str) -> Optional[str]:
    return fallback_find_library(name)


def ctypes_cdll_find_library(name: str) -> Optional[str]:
    return fallback_find_library(name)
