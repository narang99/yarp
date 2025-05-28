# yarp python discovery and exporter shim

This packages provides a convenient to install pip package which can call the main `yarp` rust binary (copied the template from `ruff`).  
Apart from this, the other main module is `yarp.discover`   

A user can hook our import discovery mechanism using 
```python
from yarp.discover import yarp_init_discovery
import os

if os.environ.get("YARP_INIT_DISCOVERY", "False") == "True":
    yarp_init_discovery()
```

These lines should run before any other code in the user's application.  
`yarp.discover` would start tracking all the imports. On shutdown, it would generate `yarp.json`, which can be passed to the `yarp` CLI to export the environment.  
`yarp.discover` does minimal static analysis of your app, it relies on discovering modules and shared libraries during runtime. To discover everything, you need to run some behaviors on your app, which you are sure would import everything in your application.  

## yarp.json

```json
{
    // shared libraries loaded using `dlopen`
    "loads": [
        {
            "path": "/users/hariomnarang/miniconda3/lib/libpango.so",
        },
    ],
    // all imported python modules, including imported c extensions
    "modules": [
        {
            // extension
            "name": "fontTools.varLib.iup",
            "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/fontTools/varLib/iup.cpython-312-darwin.so",
            "kind": "extension",
        },
        {
            // pure python
            "name": "click",
            "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/click",
            "kind": "pure",
        }
    ]
    // python interpreter information
    "python": {
        "sys": {
            "prefix": "<sys.prefix>",
            "exec_prefix": "<sys.exec_prefix>",
            "platlibdir": "<sys.platlibdir>",
            "version": {
                "major": "<sys.version_info.major>",
                "minor": "<sys.version_info.minor>",
                "abi_thread": "...",
            },
            "path": "<sys.path>",
            "executable": "<executable path>",
        }
    }
}
```
