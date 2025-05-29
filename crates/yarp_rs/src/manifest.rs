/// the module defining types for deserializing yarp.json (or called yarp manifest)
/// an example json is in this test module, code is duplicated between `python/yarp` and our crate
/// both should always be synced 
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct YarpManifest {
    pub loads: Vec<Load>,
    pub modules: Modules,
    pub python: Python,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Load {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modules {
    pub pure: Vec<Pure>,
    pub extensions: Vec<Extension>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pure {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub path: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Python {
    pub sys: Sys,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sys {
    pub prefix: String,
    pub exec_prefix: String,
    pub platlibdir: String,
    pub version: Version,
    pub path: Vec<String>,
    pub executable: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub abi_thread: String,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_deserialize() {
        let json_str = r#"
{
    "loads": [
        {
            "path": "/users/hariomnarang/miniconda3/lib/libpango.so"
        }
    ],
    "modules": {
        "pure": [
            {
                "name": "click",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/click"
            }
        ],
        "extensions": [
            {
                "name": "fontTools.varLib.iup",
                "path": "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/fontTools/varLib/iup.cpython-312-darwin.so"
            }
        ]
    },
    "python": {
        "sys": {
            "prefix": "/Users/hariomnarang/miniconda3",
            "exec_prefix": "/Users/hariomnarang/miniconda3",
            "platlibdir": "lib",
            "version": {
                "major": 3,
                "minor": 12,
                "abi_thread": ""
            },
            "path": ["/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"],
            "executable": "/Users/hariomnarang/miniconda3/bin/python"
        }
    }
}
"#;

        let manifest: super::YarpManifest =
            serde_json::from_str(json_str).expect("Failed to deserialize manifest");

        assert_eq!(manifest.loads.len(), 1);
        assert_eq!(
            manifest.loads[0].path,
            "/users/hariomnarang/miniconda3/lib/libpango.so"
        );

        assert_eq!(manifest.modules.pure.len(), 1);
        assert_eq!(manifest.modules.pure[0].name, "click");
        assert_eq!(
            manifest.modules.pure[0].path,
            "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/click"
        );

        assert_eq!(manifest.modules.extensions.len(), 1);
        assert_eq!(manifest.modules.extensions[0].name, "fontTools.varLib.iup");
        assert_eq!(
            manifest.modules.extensions[0].path,
            "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages/fontTools/varLib/iup.cpython-312-darwin.so"
        );

        assert_eq!(manifest.python.sys.prefix, "/Users/hariomnarang/miniconda3");
        assert_eq!(
            manifest.python.sys.exec_prefix,
            "/Users/hariomnarang/miniconda3"
        );
        assert_eq!(manifest.python.sys.platlibdir, "lib");
        assert_eq!(manifest.python.sys.version.major, 3);
        assert_eq!(manifest.python.sys.version.minor, 12);
        assert_eq!(manifest.python.sys.version.abi_thread, "");
        assert_eq!(manifest.python.sys.path.len(), 1);
        assert_eq!(
            manifest.python.sys.path[0],
            "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"
        );
        assert_eq!(
            manifest.python.sys.executable,
            "/Users/hariomnarang/miniconda3/bin/python"
        );
    }
}
