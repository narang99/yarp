from yarp.discover.types import (
    Modules,
    Pure,
    Extension,
    YarpDiscovery,
    Load,
    Python,
    Sys,
    Version,
)
from uuid import uuid4
import os
import sys
import subprocess
import json
from pathlib import Path


def test_yarp_discovery_serialization():
    # Create a sample YarpDiscovery object
    version = Version(major=3, minor=9, abi_thread="cp39")
    sys_info = Sys(
        prefix="/usr/local",
        exec_prefix="/usr/local",
        platlibdir="lib/python3.9/site-packages",
        version=version,
        path=["/usr/local/lib/python3.9/site-packages"],
        executable="/usr/local/bin/python3.9",
    )
    python_info = Python(sys=sys_info)

    discovery = YarpDiscovery(
        loads=[
            Load(path="/path/to/load1"),
            Load(path="/path/to/load2"),
        ],
        modules=Modules(
            pure=[Pure(name="test_module", path="/path/to/module")],
            extensions=[Extension(name="test_extension", path="/path/to/extension")],
        ),
        python=python_info,
    )

    # Serialize the object
    serialized = discovery.to_dict()

    # Verify the serialized output
    assert len(serialized["loads"]) == 2
    assert serialized["loads"][0]["path"] == "/path/to/load1"
    assert serialized["loads"][1]["path"] == "/path/to/load2"

    assert len(serialized["modules"]["pure"]) == 1
    assert len(serialized["modules"]["extensions"]) == 1
    assert serialized["modules"]["pure"][0]["name"] == "test_module"
    assert serialized["modules"]["pure"][0]["path"] == "/path/to/module"
    assert serialized["modules"]["extensions"][0]["name"] == "test_extension"
    assert serialized["modules"]["extensions"][0]["path"] == "/path/to/extension"

    python_data = serialized["python"]["sys"]
    assert python_data["prefix"] == "/usr/local"
    assert python_data["exec_prefix"] == "/usr/local"
    assert python_data["platlibdir"] == "lib/python3.9/site-packages"
    assert python_data["path"] == ["/usr/local/lib/python3.9/site-packages"]
    assert python_data["executable"] == "/usr/local/bin/python3.9"

    version_data = python_data["version"]
    assert version_data["major"] == 3
    assert version_data["minor"] == 9
    assert version_data["abi_thread"] == "cp39"


def test_yarp_init_discovery(tmp_path):
    # this test would run a python script in a subprocess
    # using the same interpreter (`sys.executable`) and the same python path (`sys.path`)
    # the script enables yarp discovery, and imports some stdlib modules, and numpy (its our test dependency)
    # it also tries to dlopen a stdlib .so file (_csv.so)
    script = tmp_path / f"{uuid4()}.py"
    yarp_json = tmp_path / f"{uuid4()}.json"
    _add_test_code_to_script_path(script)
    env = os.environ.copy()
    env["PYTHONPATH"] = os.pathsep.join(sys.path)
    env["YARP_JSON"] = str(yarp_json)
    subprocess.run(
        [sys.executable, script],
        capture_output=True,
        text=True,
        check=True,
        env=env,
    )

    with open(yarp_json) as f:
        yarp_discovery = json.load(f)

    py = yarp_discovery["python"]

    assert "sys" in py
    _sys = py["sys"]
    _sys["path"] = _sys["path"][1:]
    assert _sys["prefix"] == sys.prefix
    assert _sys["exec_prefix"] == sys.exec_prefix
    assert _sys["platlibdir"] == sys.platlibdir
    assert _sys["path"] == sys.path
    assert _sys["executable"] == sys.executable
    assert _sys["version"]["major"] == sys.version_info.major
    assert _sys["version"]["minor"] == sys.version_info.minor

    assert len(yarp_discovery["loads"]) == 1

    ext_paths = [ext["path"] for ext in yarp_discovery["modules"]["extensions"]]
    assert any(["numpy" in e for e in ext_paths])
    assert any(["datetime" in e for e in ext_paths])

    pure_paths = [p["path"] for p in yarp_discovery["modules"]["pure"]]
    assert any(["numpy" in e for e in pure_paths])
    assert any(["datetime" in e for e in pure_paths])


def _add_test_code_to_script_path(script_path: Path) -> None:
    with open(script_path, "w") as f:
        f.write(
            """
from yarp.discover import yarp_init_discovery
yarp_init_discovery()


import sys
import json
import tempfile

import datetime
import random

import numpy

def try_dlopen_csv():
    # artificially dlopen something to make it come in load
    # try importing from stdlib only so that we don't need to rely on something outside our env
    # although the functionality is precisely for stuff outside sys.path
    import sys
    from pathlib import Path
    from ctypes import CDLL

    dynload = Path([l for l in sys.path if l.endswith("lib-dynload")][0])
    csv_so_path = list(dynload.glob("_csv*"))[0]  # looks like this: _csv.cpython-312-darwin.so
    lib_csv = CDLL(str(csv_so_path))
    print(lib_csv)

    
try_dlopen_csv()

with tempfile.NamedTemporaryFile(suffix='.json', delete=False) as out:
    out.write(json.dumps({
        'imports': list(sys.modules.keys()),
        'sys_path': sys.path,
        'executable': sys.executable,
        'version': {
            'major': sys.version_info.major,
            'minor': sys.version_info.minor,
            'micro': sys.version_info.micro,
        }
    }).encode())
    print(out.name)  # Print the output file path
"""
        )
