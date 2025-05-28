"""how the output directory is structured, and functions to navigate it


The final directory structure looks like this (we figure out how to package python later)
```
run.sh # the bootstrap script which takes care of exposing an environment, this would basically set DYLD_LIBRARY_PATH and call python
python
  ...py-specific-todo-later

symlinks
  ...a directory for each linked library, each directory contains symlinks to all deps
  cv2.abi3.so
    libopencv_core.4.5.2.dylib -> reals/libopencv_core.4.5.2.dylib
reals
  r
    ...all the real libs are kept here
    ...we keep another directory `r` here so that the relative path from symlinks and from reals is the same
    cv2.abi3.so
    libopencv_core.4.5.2.dylib
ld_library_path
  l
    ...symlinks to all libs we want to expose to the dynamic linker
```
"""

from pathlib import Path
import shutil
from pylibcollect.export.types import InternalLib, SingleDep
import os
import logging

logger = logging.getLogger(__name__)


class ExportedLib:
    def __init__(self, lib: InternalLib, out_dir: Path):
        self._l = lib
        self._o = out_dir

    def copy_to_reals(self) -> None:
        loc = self.real_loc(True)
        shutil.copy2(self._l.path, loc)

    def real_loc(self, parents: bool = True) -> Path:
        loc = self.real_loc_dir() / self.lib_name()
        if parents:
            loc.parent.mkdir(parents=True, exist_ok=True)
        return loc

    def symlink_farm_loc(self, parents: bool = True) -> Path:
        loc = self.symlinks_dir() / self.lib_name()
        if parents:
            loc.mkdir(parents=True, exist_ok=True)
        return loc

    def real_loc_dir(self) -> Path:
        return self._o / "reals" / "r"

    def symlinks_dir(self) -> Path:
        return self._o / "symlinks"

    def lib_name(self) -> str:
        return Path(self._l.path).name

    def make_symlink_farm(self, deps: list[SingleDep]):
        for dep in deps:
            if not dep.bundle_able:
                continue
            other = ExportedLib(dep.dep, self._o)
            other.symlink_in_farm_of(self)

    def ld_library_path_dir(self, parents: bool = True) -> Path:
        loc = self._o / "ld_library_path" / "l"
        if parents:
            loc.mkdir(parents=True, exist_ok=True)
        return loc

    def symlink_in_ld_library_path(self) -> None:
        # all the libraries which are directly loaded using dlopen
        # should be put in the ld_library_path
        # who all we should do this for not very clear right now
        # generally i'm assuming if the shared-lib exists in site-packages
        # we dont need to put it in the ld_library_path
        # in that case we need to symlink that loc to the real_loc
        destination = self.ld_library_path_dir() / self.lib_name()
        source = Path("..") / ".." / "reals" / "r" / self.lib_name()
        source = str(source)
        os.symlink(source, destination)

    def symlink_in_farm_of(self, other: "ExportedLib") -> None:
        # symlink to our lib is created at the destination
        # the destination is our_lib_name in other's symlinks_dir
        # we need to keep symlink relative, and the path points to our real_loc
        # as an example, symlinks/cv2.abi3.so/libopencv_core.4.5.2.dylib
        # points to ../../../reals/r/libopencv_core.4.5.2.dylib
        # here other would be cv2.abi3.so and we would be libopencv_core.4.5.2.dylib
        # here `self` is a dependency, `other` is the dependent
        destination = other.symlink_farm_loc() / self.lib_name()
        source = Path("..") / ".." / "reals" / "r" / self.lib_name()
        source = str(source)
        logger.debug("symlinking %s to %s", source, destination)
        os.symlink(source, destination)

        # links = [
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libswresample.5.3.100.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libavutil.59.39.100.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libvpx.9.dylib",
        #     "/usr/lib/libSystem.B.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libwebpmux.3.1.0.dylib",
        #     "/usr/lib/libiconv.2.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/liblzma.5.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libaribb24.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libdav1d.7.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libopencore-amrwb.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libsnappy.1.2.1.dylib",
        #     "/usr/lib/libc++.1.dylib",
        #     "/System/Library/Frameworks/AudioToolbox.framework/Versions/A/AudioToolbox",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libaom.3.11.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libvmaf.3.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libjxl.0.11.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libjxl_threads.0.11.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libmp3lame.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libopencore-amrnb.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libopenjp2.2.5.2.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libopus.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/librav1e.0.7.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libspeex.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libSvtAv1Enc.2.2.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libtheoraenc.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libtheoradec.1.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libogg.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libvorbis.0.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libvorbisenc.2.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libwebp.7.1.9.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libx264.164.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libx265.212.dylib",
        #     "/usr/lib/libz.1.dylib",
        #     "/System/Library/Frameworks/VideoToolbox.framework/Versions/A/VideoToolbox",
        #     "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation",
        #     "/System/Library/Frameworks/CoreMedia.framework/Versions/A/CoreMedia",
        #     "/System/Library/Frameworks/CoreVideo.framework/Versions/A/CoreVideo",
        #     "/System/Library/Frameworks/CoreServices.framework/Versions/A/CoreServices",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libsoxr.0.1.2.dylib",
        #     "/Users/hariomnarang/miniconda3/envs/linker/lib/python3.12/site-packages/cv2/.dylibs/libX11.6.dylib",
        # ]
