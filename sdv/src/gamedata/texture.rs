use anyhow::{anyhow, Context, Result};
use std::{fs::File, io::BufReader, path::Path};
use xnb::Xnb;

pub struct Texture {
    pub texture: xnb::value::Texture,
}

impl Texture {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<Self> {
        let f = File::open(file).context("Can't open fish file")?;
        let mut r = BufReader::new(f);
        let xnb = Xnb::new(&mut r).context("Can't parse fish xnb file")?;

        let xnb::Value::Texture(texture) = xnb.content else {
		return Err(anyhow!("Loaded XNB file is not a texture"));
	};

        Ok(Self { texture })
    }
}
