
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class Load:
    path: str
    
    def to_dict(self) -> dict[str, Any]:
        return {"path": self.path}


@dataclass(frozen=True)
class Pure:
    name: str
    path: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "path": self.path,
        }


@dataclass(frozen=True)
class Extension:
    name: str
    path: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "path": self.path,
        }


@dataclass(frozen=True)
class Modules:
    extensions: list[Extension]
    pure: list[Pure]
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "extensions": [ext.to_dict() for ext in self.extensions],
            "pure": [pure.to_dict() for pure in self.pure]
        }


@dataclass(frozen=True)
class Version:
    major: int
    minor: int
    abi_thread: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "major": self.major,
            "minor": self.minor,
            "abi_thread": self.abi_thread
        }


@dataclass(frozen=True)
class Sys:
    prefix: str
    exec_prefix: str
    platlibdir: str
    version: Version
    path: list[str]
    executable: str
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "prefix": self.prefix,
            "exec_prefix": self.exec_prefix,
            "platlibdir": self.platlibdir,
            "version": self.version.to_dict(),
            "path": self.path,
            "executable": self.executable
        }


@dataclass(frozen=True)
class Python:
    sys: Sys
    
    def to_dict(self) -> dict[str, Any]:
        return {"sys": self.sys.to_dict()}


@dataclass(frozen=True)
class YarpDiscovery:
    loads: list[Load]
    modules: Modules
    python: Python
    
    def to_dict(self) -> dict[str, Any]:
        return {
            "loads": [load.to_dict() for load in self.loads],
            "modules": self.modules.to_dict(),
            "python": self.python.to_dict()
        }