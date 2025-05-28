# Algorithm

- we have sys.path, explicitly imported so libs, imported packages, sys.prefix, sys.exec_prefix
- for all existing directories in sys.path, copy them to dist
- does it make sense to keep `prefix` and `exec_prefix` separate?
    - what if `prefix` and `exec_prefix` are different in the dev machine?
    - in this case, we can simply take `lib-dynload` and dump it in the correct location, its fine

```
dist
    symlinks
        cv2.abi3.so 
            libavcodec.so (symlink to ../../reals/r/libavcodec.so)
    reals
        r
            cv2.abi3.so (actual file, everything in symlink file points to this)
            libpango.so
            libavcodec.so (all load commands are to ../../symlinks/libavcodec.so)
    ld_library_paths
        l
            libpango.so (../../reals/r/libpango.so)
    python
        bin
            python
        lib
            python<major>.<minor><abi-thread>
                os.py
                lib-dynload
    site-packages
        p1
            torch
            ...
        p2
            numpy
            ...
```

- getting `abi_thread`
```python
import sys
if hasattr(sys, 'abiflags') and 't' in sys.abiflags:
    abi_thread = 't'
else:
    abi_thread = ''
```

Algorithm:
- Go through all `sys.path` directories
    - If it is lib-dynload or pythons stdlib, copy the whole thing to its correct location
    - for other packages
        - go through all the imported modules
            - find the site-packages it came from
                - if the module is part of stdlib imports, ignore it (has already been copied in the first step)
                - else if part of a site-package: copy to the correct destination in dist
                - else: panic/return error, found a package not in site-packages (will need special handling for the module from where the user runs the script)

- Go through all the dynamic library loads (dlopen or imports)
    - if they are inside site-packages or stdlib, ignore them
- copy the remaining libs in `ld_library/l`
- collate ALL the so files now from our dist folder in `python` and `ld_library_path`
- generate symlink farm
- remove the files from the tree and symlink them here