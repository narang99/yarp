"""patch dylibs with our linking convention


The linking convention is simple, first take a look at the output directory structure at `out_dir.py`
We take an example:
- `cv2.abi3.so` is a dylib, and it has a dependency on `libopencv_core.4.5.2.dylib`

The output directory structure looks like this:
```
symlinks
  cv2.abi3.so
    libopencv_core.4.5.2.dylib -> ../../../reals/r/libopencv_core.4.5.2.dylib
reals
  r
    cv2.abi3.so
    libopencv_core.4.5.2.dylib
```


# cv2.abi3.so
Now `cv2.abi3.so` needs to be pointed to its symlink farm, where all its dependencies are present
We set the rpath of `cv2.abi3.so` to `@loader_path/../../../symlinks/cv2.abi3.so`
All load commands of `cv2.abi3.so` need to be patched to use rpath now, something like this:
```
LC_LOAD_DYLIB @rpath/libopencv_core.4.5.2.dylib
```

# libopencv_core.4.5.2.dylib
Now that libopencv_core.4.5.2.dylib is a dependency of cv2.abi3.so
it's `LC_ID_DYLIB` needs to be patched to `@rpath/libopencv_core.4.5.2.dylib` (the same string that is used to load the file)


Algorithm:
- All loadable libraries have their `LC_ID_DYLIB` set to `@rpath/lib_name`
- All libraries have their `LC_RPATH` set to `@loader_path/../../../symlinks/lib_name`
- All load commands are of the form `LC_LOAD_DYLIB @rpath/dep.lib_name`
"""

from pathlib import Path
import subprocess
from typing import Optional
import logging

from pylibcollect.export.pkg.out_dir import ExportedLib
from pylibcollect.export.types import InternalLib, LibDep, SingleDep
from pylibcollect.export.sh import quote

logger = logging.getLogger(__name__)


class LinkPatcher:
    def __init__(self, lib: InternalLib, out_dir: Path):
        self._o = out_dir
        self._l = lib
        self._e = ExportedLib(self._l, self._o)

    def set_rpath(self, old_rpath: Optional[str]) -> None:
        cmd = self._get_rpath_change_cmd(old_rpath)
        logger.debug("patch-rpath: %s", cmd)
        subprocess.run(cmd, check=True, shell=True, capture_output=True)

    def rpath(self) -> str:
        return f"@loader_path/../../symlinks/{self._e.lib_name()}"

    def _get_rpath_change_cmd(self, old_rpath: Optional[str]) -> str:
        real_loc = self._e.real_loc()
        new_rpath = self.rpath()
        cmd = self._get_rpath_change_snippet(old_rpath, new_rpath)
        cmd = f"{cmd} {quote(real_loc)}"
        return cmd

    def _get_rpath_change_snippet(self, old_rpath: Optional[str], new_rpath: str):
        if old_rpath:
            return f"install_name_tool -rpath {quote(old_rpath)} {quote(new_rpath)}"
        else:
            return f"install_name_tool -add_rpath {quote(new_rpath)}"

    def dylib_id(self) -> str:
        return f"@rpath/{self._e.lib_name()}"

    def set_lc_dylib_id(self) -> None:
        lib_id = self.dylib_id()
        cmd = f"install_name_tool -id {quote(lib_id)} {quote(self._e.real_loc())}"
        logger.debug("patch-dylib-id: %s", cmd)
        subprocess.run(cmd, check=True, shell=True, capture_output=True)

    def patch_load_cmd(self, original_load_path: str, other_lib: InternalLib) -> None:
        new = LinkPatcher(other_lib, self._o).dylib_id()
        cmd = f"install_name_tool -change {quote(original_load_path)} {quote(new)} {quote(self._e.real_loc())}"
        logger.debug("patch-load-cmd: %s", cmd)
        subprocess.run(cmd, check=True, shell=True, capture_output=True)

    def patch_load_cmds(self, deps: list[SingleDep]) -> None:
        for dep in deps:
            self.patch_load_cmd(dep.original_load_path, dep.dep)

    def resign(self) -> None:
        cmd = f"codesign -s - -f {quote(self._e.real_loc())}"
        logger.debug("resigning: %s", cmd)
        subprocess.run(cmd, check=True, shell=True, capture_output=True)