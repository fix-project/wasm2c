mod module;
mod operators;
mod script;

pub use module::wasm2c;
pub use script::wast2c;

use anyhow::Result;
use std::io::Write;

pub enum OutputSpec<T: Write> {
    Unified(T),
    Split { hh_include: String, hh: T, cc: T },
}

impl<T: Write> OutputSpec<T> {
    pub fn header(&mut self) -> &mut T {
        match self {
            OutputSpec::Unified(hh) | OutputSpec::Split { hh, .. } => hh,
        }
    }

    pub fn body(&mut self) -> &mut T {
        match self {
            OutputSpec::Unified(cc) | OutputSpec::Split { cc, .. } => cc,
        }
    }
}

pub fn print_includes(out: &mut impl Write) -> Result<()> {
    module::print_header_includes(out)?;
    module::print_source_includes(out)?;
    script::print_includes(out)?;
    Ok(())
}
