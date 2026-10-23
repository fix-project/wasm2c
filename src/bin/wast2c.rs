use anyhow::{Result, anyhow};
use clap::Parser;
use clio::{Input, Output};
use std::{ffi::OsStr, io::Read};
use wasm2c::{OutputSpec, wast2c};

#[derive(Parser)]
struct Args {
    src: Input,

    dest_header: Output,

    dest_body: Output,

    /// Whether or not "confusing unicode characters" are allowed to be present
    /// in `*.wast` files.
    ///
    /// The default for this is `false`.
    #[clap(long)]
    allow_confusing_unicode: bool,
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    let mut output = OutputSpec::Split {
        hh_include: args
            .dest_header
            .path()
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("invalid filename: {}", args.dest_header.path()))?
            .to_string(),
        hh: args.dest_header,
        cc: args.dest_body,
    };

    let mut script = String::new();
    args.src.read_to_string(&mut script)?;

    wast2c(&script, &mut output, args.allow_confusing_unicode)?;
    Ok(())
}
