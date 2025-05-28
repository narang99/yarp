"main entrypoint for exporting an installable python package given a closure"

from pathlib import Path
import shutil
from pylibcollect.export.pkg.link import LinkPatcher
from pylibcollect.export.pkg.out_dir import ExportedLib
from pylibcollect.export.types import DepClosure, InternalLib, LibDep
import logging

logger = logging.getLogger(__name__)


def export_py_app(closure: DepClosure, out_dir: Path):
    logger.info("exporting closure to %s", out_dir)
    logger.info("clearing out_dir: %s", out_dir)
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    _make_reals(closure, out_dir)
    _make_symlink_farm(closure, out_dir)
    _patch_dylibs(closure, out_dir)


def _make_reals(closure: DepClosure, out_dir: Path):
    logger.info("copying closure to %s", out_dir)
    for lib in closure.bundle_able_deps().keys():
        exported = ExportedLib(lib, out_dir)
        exported.copy_to_reals()


def _make_symlink_farm(closure: DepClosure, out_dir: Path):
    logger.info("generating symlink farm for dependency resolution")
    for lib, lib_dep in closure.bundle_able_deps().items():
        exported = ExportedLib(lib, out_dir)
        exported.make_symlink_farm(lib_dep.deps)


def _patch_dylibs(closure: DepClosure, out_dir: Path):
    logger.info("patching dylibs to fix links")
    for lib, lib_dep in closure.bundle_able_deps().items():
        _patch_dylib(lib, lib_dep, out_dir)

def _patch_dylib(lib: InternalLib, lib_dep: LibDep, out_dir: Path):
    dyld_patcher = LinkPatcher(lib, out_dir)
    dyld_patcher.set_rpath(lib_dep.rpath)
    dyld_patcher.set_lc_dylib_id()
    bundle_able_deps = [d for d in lib_dep.deps if d.bundle_able]
    dyld_patcher.patch_load_cmds(bundle_able_deps)
    dyld_patcher.resign()
