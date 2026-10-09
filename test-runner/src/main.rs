mod jit;
mod wast;

use anyhow::{Result, ensure};
use clap::Parser;
use clio::{ClioPath, has_extension};
use std::collections::BTreeSet;
use std::fmt::Write;
use std::{fs, panic};

const TESTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
const PASSING: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/passing.txt");

#[derive(Parser)]
struct Args {
    // Argument passing in path to a directory/file
    #[arg(value_parser = clap::value_parser!(ClioPath).exists())]
    path: Option<ClioPath>,
    // Argument updating passing.txt with pasing directives
    #[arg(long, conflicts_with = "path")]
    update: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    // Entire test suite is being run if no path is passed
    let full_suite = args.path.is_none();
    let path = match args.path {
        Some(path) => path,
        None => ClioPath::new(TESTS)?,
    };
    // Run every file with .wast extension as test
    let mut files = path.files(has_extension("wast"))?;
    if full_suite {
        let spec = std::path::Path::new(TESTS).join("spec");
        // Filter out non-core tests
        files.retain(|file| !file.starts_with(&spec) || file.parent() == Some(spec.as_path()));
        ensure!(
            files.iter().any(|file| file.starts_with(&spec)),
            "spec tests missing. run `git submodule update --init`"
        );
    }
    files.sort_by(|a, b| a.path().cmp(b.path()));
    ensure!(!files.is_empty(), "no .wast files found");

    let mut results = String::new();
    let mut passing = BTreeSet::new();
    for file in &files {
        let name = file.strip_prefix(TESTS).unwrap_or(file.path()).display();
        let text = fs::read_to_string(file.path())?;
        match panic::catch_unwind(|| wast::run_test(&text)) {
            Ok(Ok(outcomes)) => {
                for ((line, column), passed) in outcomes {
                    let id = format!("{name}:line {line}:column {column}");
                    let status = if passed { "pass" } else { "fail" };
                    writeln!(results, "{id} -- {status}")?;
                    if passed {
                        passing.insert(id);
                    }
                }
            }
            _ => writeln!(results, "{name} error")?,
        }
    }

    ensure!(!results.is_empty(), "no WAST directives");
    print!("{results}");

    if full_suite {
        let expected = fs::read_to_string(PASSING)?;
        let regressions: Vec<_> = expected
            .lines()
            .filter(|id| !passing.contains(*id))
            .collect();
        ensure!(
            regressions.is_empty(),
            "previously passing directives failed:\n{}",
            regressions.join("\n")
        );

        // Update passing.txt
        if args.update {
            let mut updated = String::new();
            for id in passing {
                writeln!(updated, "{id}")?;
            }
            fs::write(PASSING, updated)?;
        }
    }
    Ok(())
}
