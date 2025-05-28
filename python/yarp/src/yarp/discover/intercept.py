import importlib.abc
import json
import sys
import os
import atexit
from yarp.discover.monkeypatch import kwarg_else_arg, try_monkey_patch
from yarp.discover.types import *

DUMP_LOC_ENV_VAR = "YARP_JSON"
DEFAULT_LOC = "yarp.json"


LOADS: set[Load] = set()
PURE_MODULES: set[Pure] = set()
EXTS: set[Extension] = set()


def register_import_watcher(add_pure_callback, add_ext_callback):
    class ImportWatcher(importlib.abc.MetaPathFinder):
        def find_spec(self, fullname, path, target=None):
            # We delegate finding this import to other finders
            # if any finder returns a spec, pass a `yarp.types.Module` to callback
            spec = None
            for finder in sys.meta_path:
                if finder is not self:
                    try:
                        spec = finder.find_spec(fullname, path, target)
                        if spec is not None:
                            break
                    except (ImportError, AttributeError):
                        continue

            if spec is not None and spec.origin:
                if self._is_dyn_lib(spec.origin):
                    add_ext_callback(
                        Extension(name=spec.name, path=spec.origin),
                    )
                else:
                    add_pure_callback(Pure(name=spec.name, path=spec.origin))
            return None

        def _is_dyn_lib(self, path):
            return path.endswith(".dylib") or path.endswith(".so")

    sys.meta_path.insert(0, ImportWatcher())


def monkey_patch_dlopen(add_lib_callback):
    try_monkey_patch(
        "ctypes", ["cdll", "LoadLibrary"], add_lib_callback, kwarg_else_arg("name", 0)
    )
    try_monkey_patch(
        "ctypes", ["CDLL", "__init__"], add_lib_callback, kwarg_else_arg("name", 1)
    )
    try_monkey_patch(
        "cffi", ["api", "FFI", "dlopen"], add_lib_callback, kwarg_else_arg("name", 1)
    )


# TODO: this is a hack, we should be able to get the absolute path from dlopen
# use _dyld_image_count in the exit handler and "guess" the absolute path for all relative paths
# for now, I'm just using this to ignore relative paths, we just move on
# I'll need to import a shared-lib for this
def _rm_rel_paths(libs: set, get_path) -> None:
    to_rm = set()
    for lib in libs:
        if not os.path.isabs(get_path(lib)):
            to_rm.add(lib)
    for lib in to_rm:
        libs.remove(lib)


def python_props() -> Python:
    abi_thread = "t" if hasattr(sys, "abiflags") and "t" in sys.abiflags else ""

    return Python(
        sys=Sys(
            prefix=sys.prefix,
            exec_prefix=sys.exec_prefix,
            platlibdir=sys.platlibdir,
            version=Version(
                major=sys.version_info.major,
                minor=sys.version_info.minor,
                abi_thread=abi_thread,
            ),
            path=sys.path,
            executable=sys.executable,
        )
    )


def exit_handler():
    from copy import deepcopy

    print(LOADS, PURE_MODULES, EXTS)
    loads, pure_modules, exts = deepcopy(LOADS), deepcopy(PURE_MODULES), deepcopy(EXTS)
    _rm_rel_paths(loads, lambda l: l.path)
    _rm_rel_paths(exts, lambda l: l.path)

    dump_loc = os.environ.get(DUMP_LOC_ENV_VAR, DEFAULT_LOC)
    payload = YarpDiscovery(
        loads=list(loads),
        modules=Modules(extensions=list(exts), pure=list(pure_modules)),
        python=python_props(),
    )
    with open(dump_loc, "w") as f:
        json.dump(payload.to_dict(), f)


def add_pure_callback(lib: Pure):
    PURE_MODULES.add(lib)


def add_extension_callback(lib: Extension):
    EXTS.add(lib)


def add_load_callback(lib: Load):
    LOADS.add(lib)


_ENABLED_DISCOVERY = False


def yarp_init_discovery():
    global _ENABLED_DISCOVERY
    if _ENABLED_DISCOVERY:
        return
    _ENABLED_DISCOVERY = True
    monkey_patch_dlopen(add_load_callback)
    register_import_watcher(add_pure_callback, add_extension_callback)
    atexit.register(exit_handler)
