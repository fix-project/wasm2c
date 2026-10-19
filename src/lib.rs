mod module;
mod operators;
mod script;

pub use module::wasm2c;
pub use script::wast2c;

use std::io::Write;

pub enum OutputSpec<T: Write> {
    Unified(T),
    Split { hh_include: String, hh: T, cc: T },
}

impl<T: Write> OutputSpec<T> {
    pub fn is_unified(&self) -> bool {
        match self {
            OutputSpec::Unified(_) => true,
            OutputSpec::Split { .. } => false,
        }
    }

    pub fn header(&mut self) -> &mut T {
        match self {
            OutputSpec::Unified(hh) => hh,
            OutputSpec::Split { hh, .. } => hh,
        }
    }

    pub fn body(&mut self) -> &mut T {
        match self {
            OutputSpec::Unified(hh) => hh,
            OutputSpec::Split { cc, .. } => cc,
        }
    }
}
