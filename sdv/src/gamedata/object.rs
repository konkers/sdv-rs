use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use std::{convert::TryInto, fs::File, io::BufReader, path::Path};
use xnb::Xnb;

pub use xnb::value::ObjectData as Object;

pub fn load_objects<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, Object>> {
    let file = file.as_ref();
    let f = File::open(file).context(anyhow!("Can't open object file {}", file.display()))?;
    let mut r = BufReader::new(f);
    let xnb =
        Xnb::new(&mut r).context(anyhow!("Can't parse object xnb file {}", file.display()))?;

    xnb.content.try_into()
}

#[cfg(test)]
mod tests {}
