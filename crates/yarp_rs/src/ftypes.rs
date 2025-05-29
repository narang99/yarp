use anyhow::Result;

use crate::node::{DistFile};

#[derive(Debug)]
struct PythonExe {

}

impl DistFile for PythonExe {
    fn deps(&self) -> Result<Vec<crate::node::DistNode>> {
        unimplemented!() 
    }
}