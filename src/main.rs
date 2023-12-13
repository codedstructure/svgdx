use anyhow::Result;

fn main() -> Result<()> {
    svgd::run(svgd::get_config()?)?;

    Ok(())
}
