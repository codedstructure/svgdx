use svgdx::Result;

use svgdx_cli::{get_config, run};

fn main() -> Result<()> {
    run(get_config()?)?;

    Ok(())
}
