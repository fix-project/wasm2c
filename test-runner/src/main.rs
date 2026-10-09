mod jit;
mod wast;

use anyhow::{Result, ensure};
use clap::Parser;
use clio::{ClioPath, has_extension};
use std::fmt::Write;
use std::{fs, panic};

const TESTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/samples");

#[derive(Parser)]
struct Args {
    #[arg(
        value_parser = clap::value_parser!(ClioPath).exists(),
        default_value = TESTS
    )]
    path: ClioPath,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut files = args.path.files(has_extension("wast"))?;
    files.sort_by(|a, b| a.path().cmp(b.path()));
    ensure!(!files.is_empty(), "no .wast files found");

    let mut results = String::new();
    for file in &files {
        let name = file.strip_prefix(TESTS).unwrap_or(file.path()).display();
        let text = fs::read_to_string(file.path())?;
        match panic::catch_unwind(|| wast::run_test(&text)) {
            Ok(Ok(outcomes)) => {
                for ((line, column), passed) in outcomes {
                    let status = if passed { "pass" } else { "fail" };
                    writeln!(results, "{name}:{line}:{column} {status}")?;
                }
            }
            _ => writeln!(results, "{name} error")?,
        }
    }

    ensure!(!results.is_empty(), "no WAST directives");
    print!("{results}");
    Ok(())
}
