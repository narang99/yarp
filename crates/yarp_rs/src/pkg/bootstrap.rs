// generating the bootstrap script

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use log::info;

use crate::{
    gather::PythonPathComponent,
    manifest::Version,
    pkg::paths::{lib_dynload_relative_path, site_pkgs_relative_path, stdlib_relative_path},
};

const BOOTSTRAP_SCRIPT: &str = r#"
#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "bootstrap directory: $SCRIPT_DIR"

ORIGINAL_DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="$SCRIPT_DIR/lib/l:$ORIGINAL_DYLD_LIBRARY_PATH"
echo "DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"

# SITE_PKG_REL_PATHS=("site-packages/numpy" "site-packages/pandas" "site-packages/scipy")
SITE_PKG_REL_PATHS={{SITE_PKGS_REPLACEMENT}}

export PYTHONPATH=""
for path in "${SITE_PKG_REL_PATHS[@]}"; do 
    export PYTHONPATH="$PYTHONPATH:$SCRIPT_DIR/$path"
done

echo "PYTHONPATH=$PYTHONPATH"

exec "$SCRIPT_DIR/python/bin/python" "$@"
"#;

pub fn write_bootstrap_script(
    dist: &PathBuf,
    comps: &Vec<PythonPathComponent>,
    version: &Version,
) -> Result<()> {
    let script_path = dist.join("bootstrap.sh");
    info!("writing bootstrap script at {}", script_path.display());
    info!("python path components: {:?}", comps);
    let comps_array = python_path_from_components(comps, version).with_context(|| {
        format!(
            "failed in generated PYTHONPATH, components={:?} version={:?}",
            comps, version
        )
    })?;
    let script = BOOTSTRAP_SCRIPT.replace("{{SITE_PKGS_REPLACEMENT}}", &comps_array);
    fs::write(script_path, script)?;
    info!("bootstrap script written");
    Ok(())
}

fn python_path_from_components(
    comps: &Vec<PythonPathComponent>,
    version: &Version,
) -> Result<String> {
    let mut res = Vec::new();
    let stdlib_rel_path = path_buf_to_str(&stdlib_relative_path(version))?;
    let lib_dynload_rel_path = path_buf_to_str(&lib_dynload_relative_path(version))?;
    for comp in comps {
        match comp {
            PythonPathComponent::RelativeToLibDynLoad { rel_path } => {
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", lib_dynload_rel_path, rel_path));
            }
            PythonPathComponent::RelativeToStdlib { rel_path } => {
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", stdlib_rel_path, rel_path));
            }
            PythonPathComponent::TopLevel { alias } => {
                let site_pkgs_path = path_buf_to_str(&site_pkgs_relative_path(alias))?;
                res.push(site_pkgs_path);
            }
            PythonPathComponent::RelativeToSitePkg {
                top_level_alias,
                rel_path,
            } => {
                let site_pkgs_path = path_buf_to_str(&site_pkgs_relative_path(&top_level_alias))?;
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", site_pkgs_path, rel_path));
            }
        }
    }
    let bash_array_contents = res
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<String>>()
        .join(" ");

    let bash_array = format!("({})", bash_array_contents);
    Ok(bash_array)
}

fn path_buf_to_str(b: &PathBuf) -> Result<String> {
    let p = b.to_str().ok_or_else(|| {
        anyhow!(
            "failed in converting relative path to string {}",
            b.display()
        )
    })?;
    Ok(p.to_string())
}
