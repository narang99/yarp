from dataclasses import dataclass, field
from typing import Literal, Optional, NewType, Union

from pylibcollect.types import Lib, SimplePath


@dataclass(frozen=True)
class LinkedLib:
    kind: Literal["linked_lib"]
    path: SimplePath


# these are libraries which are installed with the system
# it doesn't look like i can package these libs with the app itself
# paths like /usr/lib/libSystem.B.dylib don't even exist on the file-system
# the OS shared cache takes care of them
@dataclass(frozen=True)
class SystemLinkedLib(LinkedLib):
    kind: Literal["system_linked_lib"]


# same problem as system libraries i think, but needs more work
@dataclass(frozen=True)
class SystemFrameworkLib(LinkedLib):
    kind: Literal["system_framework_lib"]


InternalLib = Union[Lib, LinkedLib, SystemLinkedLib, SystemFrameworkLib]


@dataclass
class SingleDep:
    dep: InternalLib
    bundle_able: bool
    load_cmd: str
    original_load_path: str


@dataclass
class LibDep:
    rpath: Optional[str] = None
    bundle_able: bool = True
    deps: list[SingleDep] = field(default_factory=list)


@dataclass
class DepClosure:
    # this is just the whole set of dependencies
    all_deps: dict[InternalLib, LibDep] = field(default_factory=dict)

    def bundle_able_deps(self) -> dict[InternalLib, LibDep]:
        return {
            lib: dep
            for lib, dep in self.all_deps.items()
            if dep.bundle_able
        }


@dataclass(frozen=True)
class DynLibClientCtx:
    executable_path: SimplePath
    loader_path: SimplePath
    rpath: Optional[SimplePath] = None


def is_bundle_able(lib: InternalLib) -> bool:
    "system libraries cannot be bundled, mac stores them in shared cache and these libs are not accessible using FS"
    if isinstance(lib, SystemFrameworkLib) or isinstance(lib, SystemLinkedLib):
        return False
    else:
        return True
