mod jit;

use crate::jit::run;
use anyhow::{Result, ensure};
use clap::Parser;
use clio::{ClioPath, has_extension};
use std::fs;
use wasm2c::{OutputSpec, wast2c};

#[derive(Parser)]
struct Args {
    // Argument passing in path to a directory/file
    #[arg(value_parser = clap::value_parser!(ClioPath).exists())]
    path: ClioPath,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Run every file with .wast extension as test
    let mut files = args.path.files(has_extension("wast"))?;
    files.sort_by(|a, b| a.path().cmp(b.path()));
    ensure!(!files.is_empty(), "no .wast files found");

    for file in &files {
        // TODO: use rayon to make parallel
        let script = fs::read_to_string(file.path())?;

        let mut header = Vec::new();
        let mut body = Vec::new();

        let mut output = OutputSpec::Split {
            hh_include: String::from("input.hh"),
            hh: &mut header,
            cc: &mut body,
        };

        let num_directives = wast2c(&script, &mut output)?;
        let num_successes = run(&body, &header)?;

        println!(
            "{}: {num_successes}/{num_directives} tests passed.",
            file.path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );

        // TODO: run tests
    }

    Ok(())
}
