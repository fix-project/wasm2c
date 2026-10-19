use anyhow::{Result, anyhow};
use clap::{CommandFactory, FromArgMatches, Parser};
use clio::Output;
use std::{ffi::OsStr, path::PathBuf};
use wasm_tools::InputArg;
use wasm2c::{OutputSpec, wasm2c};

#[derive(Parser)]
struct Args {
    #[clap(flatten)]
    src: InputArg,

    #[arg(num_args = 0..=2)]
    dest_files: Vec<Output>,

    #[arg(short, long)]
    name: Option<String>,
}

fn main() -> Result<()> {
    let matches = Args::command().get_matches();
    let args = Args::from_arg_matches(&matches)?;

    let mut output = match args.dest_files.as_slice() {
        [] => OutputSpec::Unified(clio::Output::std()),
        [out] => OutputSpec::Unified(out.to_owned()),
        [hh, cc] => OutputSpec::Split {
            hh_include: hh
                .path()
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or(anyhow!("invalid filename: {}", hh.path()))?
                .to_string(),
            hh: hh.to_owned(),
            cc: cc.to_owned(),
        },
        _ => unimplemented!("unexpected argument count from clap"),
    };

    let name = args.name.as_deref().unwrap_or_else(|| {
        match matches
            .get_one("input")
            .and_then(|x: &PathBuf| x.file_prefix().and_then(OsStr::to_str))
        {
            None => "module",
            Some("-") => "stdin",
            Some(x) => x,
        }
    });

    wasm2c(name, &args.src.get_binary_wasm(None)?, &mut output)?;
    Ok(())
}
