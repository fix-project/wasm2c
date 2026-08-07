use clap::Parser as ClapParser;
use std::fs::File;
use std::io::{Read, Write};
use buffer_redux::{BufReader, BufWriter};
use clio::*;
use anyhow::{anyhow, Result};

/* SECTION 1: I/O */
#[derive(ClapParser)]
pub struct Args {
    #[clap(value_parser, default_value="-")]
    pub src: clio::Input,

    #[clap(long, short, value_parser = clap::value_parser!(ClioPath).exists().is_dir(), default_value = ".")]
    pub dest: clio::ClioPath,

    #[clap(long, short, required_unless_present = "src")]
    pub name: Option<String>,
}

pub struct Config<R: Read> {
    pub reader: BufReader<R>,
    pub dest_dir: ClioPath,
    pub name: String,
}

/* SECTION 1.1: INPUT */
pub fn get_config() -> Result<Config<Box<dyn Read>>> {
    let args = Args::parse();
    Ok(Config{ 
        name: match args.name {
            Some(name) => name,
            None => get_src_name(args.src.path())?
        },
        reader: BufReader::new_ringbuf(Box::new(args.src)), 
        dest_dir: args.dest, 
    })
}

fn get_src_name(path: &ClioPath) -> Result<String> {
    let file_name = path.file_stem().ok_or_else(|| anyhow!("expected file name"))?.to_string_lossy().to_string();
    Ok(file_name)
}

/* SECTION 1.2: OUTPUT */

pub struct Output<H: Write, C: Write> {
    pub header: BufWriter<H>,
    pub source: BufWriter<C>,
    pub name: String,
}

pub fn get_output(config: &mut Config<impl Read>) -> Result<Output<Box<dyn Write>, Box<dyn Write>>> {
    let header_file = format!("{}.hh", config.name);
    let source_file = format!("{}.cc", config.name);
    let h = File::create(config.dest_dir.join(&header_file).path())?;
    let s = File::create(config.dest_dir.join(&source_file).path())?;

    Ok(Output {
        header: BufWriter::new_ringbuf(Box::new(h) as Box<dyn Write>),
        source: BufWriter::new_ringbuf(Box::new(s) as Box<dyn Write>),
        name: config.name.clone(),
    })
}

