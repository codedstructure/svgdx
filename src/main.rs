use anyhow::Result;

fn main() -> Result<()> {
    svgdx::run(svgdx::get_config()?)?;

    Ok(())
}
