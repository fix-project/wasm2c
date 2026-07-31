use anyhow::{Result};
use wast::Wast;
use wast::parser::{self, ParseBuffer};

fn main() -> Result<()> {
    let text = std::fs::read_to_string("test/assert.wast")?;

    let buf = ParseBuffer::new(&text)?;
    let wast = parser::parse::<Wast>(&buf)?;

    for dir in wast.directives {
        match dir {
            wast::WastDirective::Module(mut module) => {
                let bytes = module.encode()?;
                // pipe it into wasm2c
                // bytes into a redux buff??
                dbg!(bytes);
            },
            wast::WastDirective::AssertReturn { results, .. } => {
                // 
                dbg!(results);
            },
            _ => {}
        }
    }
    Ok(())
}
