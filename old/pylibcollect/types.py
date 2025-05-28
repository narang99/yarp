from dataclasses import dataclass
from typing import Literal, Union, NewType

# a fully resolved path, it should not contain any @rpath, @executable_path or @loader_path refs
SimplePath = NewType("SimplePath", str)


@dataclass(frozen=True)
class ImportedDynLib:
    kind: Literal["imported"]
    name: str
    path: SimplePath

    def to_dict(self):
        return {
            "kind": self.kind,
            "name": self.name,
            "path": self.path,
        }


@dataclass(frozen=True)
class LdLoadedLib:
    kind: Literal["dlopened"]
    path: SimplePath

    def to_dict(self):
        return {
            "kind": self.kind,
            "path": self.path,
        }


Lib = Union[ImportedDynLib, LdLoadedLib]


@dataclass
class PyInterpreter:
    path: SimplePath
    prefix: SimplePath
    python_path: list[SimplePath]

    def to_dict(self):
        return {
            "path": self.path,
            "prefix": self.prefix,
            "python_path": self.python_path,
        }


@dataclass
class PyLibCollectPayload:
    direct_loads: list[Lib]
    py: PyInterpreter

    def to_dict(self):
        return {
            "direct_loads": [l.to_dict() for l in self.direct_loads],
            "py": self.py.to_dict(),
        }


def lib_from_dict(d: dict) -> Lib:
    if d["kind"] in ["imported", "dlopened"]:
        path = SimplePath(d["path"])
        d.pop("path")
        if d["kind"] == "imported":
            return ImportedDynLib(path=path, **d)
        elif d["kind"] == "dlopened":
            return LdLoadedLib(path=path, **d)
    raise Exception(f"Unknown lib kind: {d['kind']}")


def from_json(json_data: dict) -> PyLibCollectPayload:
    return PyLibCollectPayload(
        direct_loads=[lib_from_dict(l) for l in json_data["direct_loads"]],
        py=PyInterpreter(**json_data["py"]),
    )
