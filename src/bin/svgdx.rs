use anyhow::Result;

use svgdx::cli::{get_config, run};

fn main() -> Result<()> {
    run(get_config()?)?;

    Ok(())
}
