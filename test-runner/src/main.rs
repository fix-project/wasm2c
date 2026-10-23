mod jit;

use crate::jit::run;
use anyhow::{Result, ensure};
use clap::Parser;
use clio::ClioPath;
use std::{ffi::OsStr, fs, path::PathBuf};
use walkdir::WalkDir;
use wasm2c::{OutputSpec, wast2c};

#[derive(Parser)]
struct Args {
    /// One or more wast files, or directories containing wast files (will not descend directories)
    #[arg(required = true, value_parser = clap::value_parser!(ClioPath).exists())]
    paths: Vec<ClioPath>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Collect all scripts
    let mut files: Vec<PathBuf> = Vec::new();
    for path in &args.paths {
        for entry in WalkDir::new(path.as_os_str()).max_depth(1) {
            let entry = entry?;
            if entry.file_type().is_file()
                && let Some("wast") = entry.path().extension().and_then(OsStr::to_str)
            {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();
    ensure!(!files.is_empty(), "no .wast files found");

    for file in &files {
        // TODO: use rayon to make parallel
        let script = fs::read_to_string(file)?;

        let mut header = Vec::new();
        let mut body = Vec::new();

        let mut output = OutputSpec::Split {
            hh_include: String::from("input.hh"),
            hh: &mut header,
            cc: &mut body,
        };

        let name = file.file_name().unwrap_or_default().to_string_lossy();

        match wast2c(&script, &mut output) {
            Ok(num_directives) => {
                eprint!("Running {name}... ");
                let num_successes = run(&body, &header)?;
                eprintln!("done.");

                println!("{name}: {num_successes}/{num_directives} tests passed.",);
            }
            Err(e) => {
                println!("{name} error: {e}.",);
            }
        }
    }

    Ok(())
}
