use anyhow::Result;

fn main() -> Result<()> {
    let mut config = wasm2c::config::get_config()?;
    wasm2c::codegen::compile(&mut config)?;
    Ok(())
}
