from functools import partial
from pathlib import Path
import logging
from typing import Iterator, Optional

from pylibcollect.export.types import (
    DynLibClientCtx,
    LinkedLib,
    DepClosure,
    SimplePath,
    LibDep,
    InternalLib,
    SingleDep,
    SystemFrameworkLib,
    SystemLinkedLib,
    is_bundle_able,
)
import json
import subprocess
import shutil


logger = logging.getLogger(__name__)


SYS_LIBS = set(
    [
        "/usr/lib/libc++.1.dylib",
        "/usr/lib/libiconv.2.dylib",
        "/usr/lib/libz.1.dylib",
        "/usr/lib/libxml2.2.dylib",
        "/usr/lib/libbz2.1.0.dylib",
        "/usr/lib/libexpat.1.dylib",
        "/usr/lib/libffi.dylib",
        "/usr/lib/libobjc.A.dylib",
        "/usr/lib/libSystem.B.dylib",
    ]
)


def generate_mac_closure(
    libs: list[InternalLib], actual_executable_path: SimplePath
) -> DepClosure:
    closure = DepClosure()
    for lib in libs:
        _add_single_lib_closure(lib, actual_executable_path, closure)
    return closure


def _add_single_lib_closure(
    lib: InternalLib, actual_executable_path: SimplePath, closure: DepClosure
) -> None:
    if lib in closure.all_deps:
        return
    if not is_bundle_able(lib):
        closure.all_deps[lib] = LibDep(bundle_able=False)
        return
    if not Path(lib.path).exists():
        raise Exception(f"Library {lib.path} does not exist")
        # logger.error(f"Library {lib.path} does not exist")
        # closure.all_deps[lib] = LibDep()
        # return
    rpath, linked_libs = get_mac_linked_libs(lib, actual_executable_path)
    lib_dep = LibDep(rpath=rpath, deps=linked_libs)
    closure.all_deps[lib] = lib_dep

    for linked in linked_libs:
        _add_single_lib_closure(linked.dep, actual_executable_path, closure)


def get_mac_linked_libs(
    lib: InternalLib, actual_executable_path: SimplePath
) -> tuple[Optional[str], list[SingleDep]]:
    executable_path = SimplePath(str(Path(actual_executable_path).parent))
    data = _get_ipsw_info(lib)
    ctx = _prepare_ctx(lib, data, executable_path)
    sections = list(_get_load_cmds(data, ["LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB"]))
    return _get_raw_rpath(data, lib.path), [_get_linked_lib(s, lib, ctx) for s in sections]


def _get_linked_lib(section: dict, lib: InternalLib, ctx: DynLibClientCtx) -> SingleDep:
    original_name = section["name"]
    path = _resolve_load_cmd_path(original_name, lib.path, ctx)
    load_cmd = section["load_cmd"]
    bundle_able = True
    init = None
    if path.startswith("/System/Library/Frameworks"):
        init = partial(SystemFrameworkLib, kind="system_framework_lib")
        bundle_able = False
    elif path in SYS_LIBS:
        init = partial(SystemLinkedLib, kind="system_linked_lib")
        bundle_able = False
    else:
        init = partial(LinkedLib, kind="linked_lib")
    linked_lib = init(path=path)
    return SingleDep(
        dep=linked_lib,
        load_cmd=load_cmd,
        original_load_path=original_name,
        bundle_able=bundle_able,
    )


def _prepare_ctx(
    lib: InternalLib, data, executable_path: SimplePath
) -> DynLibClientCtx:
    loader_path = SimplePath(str(Path(lib.path).parent))
    rpath = _get_rpath(
        data,
        lib.path,
        DynLibClientCtx(executable_path=executable_path, loader_path=loader_path),
    )
    return DynLibClientCtx(
        executable_path=executable_path, loader_path=loader_path, rpath=rpath
    )


def _get_ipsw_info(lib: InternalLib) -> dict:
    ipsw = shutil.which("ipsw")
    if not ipsw:
        raise Exception(
            "Command `ipsw` not found, install from here: https://blacktop.github.io/ipsw/"
        )
    cmd = ["ipsw", "macho", "info", "--json", lib.path]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        raise Exception(
            f"Command `ipsw macho info --json {lib.path}` failed with return code {result.returncode}"
        )
    return json.loads(result.stdout)


def _get_rpath(
    data: dict, lib_path: SimplePath, ctx: DynLibClientCtx
) -> Optional[SimplePath]:
    rpath = _get_raw_rpath(data, lib_path)
    if rpath is None:
        return None
    if rpath.startswith("@rpath"):
        raise Exception(
            f"LC_RPATH of {lib_path} starts with @rpath, this is not a valid library path, value={rpath}"
        )
    return _resolve_load_cmd_path(rpath, lib_path, ctx)


def _get_raw_rpath(data: dict, lib_path: SimplePath) -> Optional[SimplePath]:
    rpath_sections = list(_get_load_cmds(data, ["LC_RPATH"]))
    if len(rpath_sections) == 0:
        return None
    elif len(rpath_sections) > 1:
        raise Exception(
            f"Found multiple LC_RPATH sections in {lib_path}, expected 1, got {len(rpath_sections)}"
        )
    return rpath_sections[0]["path"]

def _get_load_cmds(data: dict, cmds: list[str]) -> Iterator[dict]:
    sections = data.get("loads", [])
    for section in sections:
        if section.get("load_cmd", "") in cmds:
            yield section


def _resolve_load_cmd_path(
    path: str, lib_path: str, ctx: DynLibClientCtx
) -> SimplePath:
    if path.startswith("@rpath"):
        if not ctx.rpath:
            raise Exception(
                f"Found @rpath in {path}, but no rpath was passed while resolving symbol for library={lib_path}"
            )
        return _resolve_load_cmd_path_with_prefix(path, "@rpath", ctx.rpath)
    elif path.startswith("@executable_path"):
        return _resolve_load_cmd_path_with_prefix(
            path, "@executable_path", ctx.executable_path
        )
    elif path.startswith("@loader_path"):
        return _resolve_load_cmd_path_with_prefix(path, "@loader_path", ctx.loader_path)
    else:
        return SimplePath(path)


def _resolve_load_cmd_path_with_prefix(
    path: str, prefix: str, base_path: SimplePath
) -> SimplePath:
    path = _strip_load_prefix(path, prefix)
    return _join_and_resolve(base_path, path)


def _strip_load_prefix(lib_name: str, prefix: str) -> str:
    if lib_name.startswith(prefix):
        lib_name = lib_name[len(prefix) :]
        if lib_name.startswith("/"):
            lib_name = lib_name[len("/") :]
        return lib_name
    else:
        raise Exception(f"Library name {lib_name} does not start with {prefix}")


def _join_and_resolve(base: SimplePath, next: str) -> SimplePath:
    p = (Path(base).resolve() / next).resolve()
    return SimplePath(str(p))
