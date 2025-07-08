"""Call ldd for finding the link path"""

import subprocess
import re
from pathlib import Path
from typing import Optional


def find(name: str) -> Optional[Path]:
    """
    Find a library using ldd command.
    
    Args:
        name: The library name to search for
        
    Returns:
        Path to the found library
        
    Raises:
        RuntimeError: If library is not found or path doesn't exist
        subprocess.CalledProcessError: If ldd command fails
    """
    try:
        output = subprocess.run(
            ["ldd", "-p"],
            capture_output=True,
            text=True,
            env={"LANG": "C", "LC_ALL": "C"},
            check=True
        )
        
        path = find_in_output(name, output.stdout)
        if path is None:
            return None
        if not path.exists():
            return None
        return path
        
    except subprocess.CalledProcessError as e:
        # give up
        return None


def find_in_output(name: str, output: str) -> Optional[Path]:
    """
    Parse ldd output to find a specific library.
    
    Output format:
        linux-vdso.so.1 (0x00007ffeb3bc5000)
        libcudnn.so.8 => /lib/x86_64-linux-gnu/libcudnn.so.8 (0x00007f777c1f2000)
        libcublas.so.11 => not found
        libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f777c1ec000)
        libnvinfer.so.8 => not found
        libnvinfer_plugin.so.8 => not found
        libnvonnxparser.so.8 => not found
        libcudart.so.11.0 => not found
        libz.so.1 => /lib/x86_64-linux-gnu/libz.so.1 (0x00007f777c1ce000)
        libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)
        librt.so.1 => /lib/x86_64-linux-gnu/librt.so.1 (0x00007f777c1a1000)
        libstdc++.so.6 => /lib/x86_64-linux-gnu/libstdc++.so.6 (0x00007f777bf33000)
        libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f777bde4000)
        libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f777bdbd000)
        libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f777bbcb000)
        /lib64/ld-linux-x86-64.so.2 (0x00007f777c6f8000)
    """
    for line in output.splitlines():
        if "=>" not in line:
            continue
        parts = [part.strip() for part in line.strip().split("=>")]
        if len(parts) > 1 and parts[0] == name:
            result = find_in_ldd_entry_value(parts[1])
            if result is not None:
                return result
    return None


def find_in_ldd_entry_value(value: str) -> Optional[Path]:
    """
    Parse the value part of an ldd entry.
    
    Args:
        value: String like "/lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f777c1ab000)"
        
    Returns:
        Path object if valid, None otherwise
    """
    if value == "not found":
        return None
    
    parts = value.split()
    if len(parts) > 1:
        # Check if second part starts with hex address
        if not parts[1].startswith("(0x"):
            return None
        
        try:
            return Path(parts[0])
        except (ValueError, OSError):
            return None
    
    return None