import importlib.abc
import sys
import os
import atexit
from pylibcollect.types import (
    Lib,
    ImportedDynLib,
    PyInterpreter,
    PyLibCollectPayload,
    SimplePath,
    LdLoadedLib,
)
from pathlib import Path

DUMP_LOC_ENV_VAR = "PYLIBCOLLECT_DUMP_LOC"

DETECTED_LIBS: set[Lib] = set()


def register_import_watcher(add_lib_callback):
    class ImportWatcher(importlib.abc.MetaPathFinder):
        def find_spec(self, fullname, path, target=None):
            # Let other finders attempt first
            spec = None
            for finder in sys.meta_path:
                if finder is not self:
                    try:
                        spec = finder.find_spec(fullname, path, target)
                        if spec is not None:
                            break
                    except (ImportError, AttributeError):
                        continue

            if spec is not None:
                if spec.origin and self._is_dyn_lib(spec.origin):
                    add_lib_callback(
                        ImportedDynLib(
                            kind="imported",
                            name=spec.name,
                            path=SimplePath(spec.origin),
                        )
                    )
            return None

        def _is_dyn_lib(self, path):
            return path.endswith(".dylib") or path.endswith(".so")

    sys.meta_path.insert(0, ImportWatcher())


def _get_element(mod, attrs):
    el = mod
    for attr in attrs:
        el = getattr(el, attr)
    return el


def _set_element(mod, attrs, el):
    parent = _get_element(mod, attrs[:-1])
    setattr(parent, attrs[-1], el)


def _monkey_patch(mod, attrs, add_lib_callback, args_to_path):
    try:
        original_fn = _get_element(mod, attrs)

        def new_fn(*args, **kwargs):
            path = args_to_path(*args, **kwargs)
            print("adding path path =", path, "args =", args, "kwargs =", kwargs)
            add_lib_callback(LdLoadedLib(kind="dlopened", path=path))
            return original_fn(*args, **kwargs)

        _set_element(mod, attrs, new_fn)
    except AttributeError as ex:
        print("failed in patching", mod, attrs, ex)


def _try_monkey_patch(mod, attrs, add_lib_callback, args_to_path):
    try:
        mod_ = importlib.import_module(mod)
        _monkey_patch(mod_, attrs, add_lib_callback, args_to_path)
        print("patched", mod, attrs)
    except ImportError as ex:
        print("failed in importing", mod, ex)


def _kw_var_else_a_i(var, i):
    def _f(*a, **kw):
        return kw[var] if var in kw else a[i]

    return _f


def monkey_patch(add_lib_callback):
    _try_monkey_patch(
        "ctypes", ["cdll", "LoadLibrary"], add_lib_callback, _kw_var_else_a_i("name", 0)
    )
    _try_monkey_patch(
        "ctypes", ["CDLL", "__init__"], add_lib_callback, _kw_var_else_a_i("name", 1)
    )
    _try_monkey_patch(
        "cffi", ["api", "FFI", "dlopen"], add_lib_callback, _kw_var_else_a_i("name", 1)
    )


# def monkeypatch_load_library(add_lib_callback):
#     import ctypes

#     original_fn = ctypes.cdll.LoadLibrary

#     def new_fn(*args, **kwargs):
#         add_lib_callback(LdLoadedLib(kind="dlopened", path=args[0]))
#         return original_fn(*args, **kwargs)

#     ctypes.cdll.LoadLibrary = new_fn

#     original_init = ctypes.CDLL.__init__

#     def _new_init(self, *args, **kwargs):
#         add_lib_callback(LdLoadedLib(kind="dlopened", path=args[0]))
#         original_init(self, *args, **kwargs)

#     ctypes.CDLL.__init__ = _new_init


# def _segregate_stdlib(libs: set[Lib]) -> set[Lib]:
#     without_stdlibs = (
#         lib
#         for lib in libs
#         if isinstance(lib, ImportedDynLib) and lib.name not in sys.stdlib_module_names
#     )
#     _stdlibs = (
#         lib
#         for lib in libs
#         if isinstance(lib, ImportedDynLib)
#         if lib.name in sys.stdlib_module_names
#     )
#     _stdlibs = (
#         StdImportedDynLib(kind="std_imported", name=lib.name, path=lib.path)
#         for lib in _stdlibs
#     )
#     return set(chain(without_stdlibs, _stdlibs))


def _put_all_libs_in_path(libs: set[Lib]) -> None:
    allowed_exts = set([".so", ".dylib"])
    for p in sys.path:
        p = Path(p)
        if p.exists() and p.is_dir():
            for p_ in p.glob("*"):
                if p_.suffix in allowed_exts:
                    libs.add(
                        LdLoadedLib(kind="dlopened", path=SimplePath(str(p_)))
                    )


# TODO: this is a hack, we should be able to get the absolute path from dlopen
# use _dyld_image_count in the exit handler and "guess" the absolute path for all relative paths
# for now, I'm just using this to ignore relative paths, we just move on
# I'll need to import a shared-lib for this
def _rm_rel_paths(libs: set[Lib]) -> None:
    to_rm = set()
    for lib in libs:
        if not os.path.isabs(lib.path):
            to_rm.add(lib)
    for lib in to_rm:
        libs.remove(lib)


def exit_handler():
    from copy import deepcopy

    print(DETECTED_LIBS)
    libs = deepcopy(DETECTED_LIBS)
    _put_all_libs_in_path(libs)
    _rm_rel_paths(libs)
    import os
    import json

    dump_loc = os.environ.get(DUMP_LOC_ENV_VAR, "pylibcollect.json")
    payload = PyLibCollectPayload(
        direct_loads=list(libs),
        py=PyInterpreter(
            path=SimplePath(sys.executable),
            prefix=SimplePath(sys.prefix),
            python_path=[SimplePath(p) for p in sys.path],
        ),
    )
    with open(dump_loc, "w") as f:
        json.dump(payload.to_dict(), f)


def add_lib_callback(lib: Lib):
    DETECTED_LIBS.add(lib)


_ENABLED_DISCOVERY = False


def enable_discovery():
    global _ENABLED_DISCOVERY
    if _ENABLED_DISCOVERY:
        return
    _ENABLED_DISCOVERY = True
    monkey_patch(add_lib_callback)
    register_import_watcher(add_lib_callback)
    atexit.register(exit_handler)
