// TYPES: i32, i64, f32, f64 -> uint32_t, uint64_t, float, double
// SINGLE RETURN VALUES
// EXPORT: functions only

use anyhow::{bail, anyhow, Result};
use buffer_redux::{BufReader, BufWriter};
use std::io::{Read, Write};
use std::io::{stdin, stdout};
use std::fs::File;
use std::env::{self, current_dir};
use std::path::{Path, PathBuf};
use wasmparser::{
    Chunk, Export, ExternalKind, FuncToValidate, FuncType, FuncValidatorAllocations, FunctionBody, 
    ModuleArity, Operator, Payload, TypeRef, ValType, ValidPayload, Validator, 
    WasmModuleResources
};

use clap::Parser;
use clio::*;


fn main() -> Result<()> {

    // SECTION 1: I/O
    let mut config = get_config()?;
    let mut output = get_output(&mut config)?;

    let mut input_stream = config.reader;

    // SECTION 2: BOILERPLATE
    print_includes(&mut output)?;
    print_typedefs(&mut output)?;
   
    // SECTION 3: printing program
    //
    // print_program(&mut input_stream, &mut output_stream)?;

    Ok(())
}

/* SECTION 1: I/O
 */
#[derive(clap::Parser)]
struct Args {
    #[clap(value_parser, default_value="-")]
    src: clio::Input,

    #[clap(long, short, value_parser = clap::value_parser!(ClioPath).exists().is_dir(), default_value = ".")]
    dest: clio::ClioPath,

    #[clap(long, short, required_unless_present = "src")]
    name: Option<String>,
}

struct Config<R: Read> {
    reader: BufReader<R>,
    dest_dir: ClioPath,
    name: String,
}

struct Output<H: Write, C: Write> {
    header: BufWriter<H>,
    source: BufWriter<C>,
    header_file: String,
    source_file: String,
}

/* SECTION 1.1: INPUT */
fn get_config() -> Result<Config<Box<dyn Read>>> {
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
fn get_output(config: &mut Config<impl Read>) -> Result<Output<Box<dyn Write>, Box<dyn Write>>> {
    let header_file = format!("{}.h", config.name);
    let source_file = format!("{}.cc", config.name);
    let h = File::create(config.dest_dir.join(&header_file).path())?;
    let s = File::create(config.dest_dir.join(&source_file).path())?;

    Ok(Output {
        header: BufWriter::new_ringbuf(Box::new(h) as Box<dyn Write>),
        source: BufWriter::new_ringbuf(Box::new(s) as Box<dyn Write>),
        header_file: header_file,
        source_file: source_file,
    })
}

/* SECTION 2: BOILERPLATE
 * includes and typedefs
 */
fn print_includes(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    print_header_includes(out)?;
    print_source_includes(out)?;
    Ok(())
}

fn print_header_includes(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let header_includes = ["<stdint.h>"];
    for inc in header_includes {
        writeln!(out.header, "#include {}", inc)?;
    }
    writeln!(out.header)?;
    Ok(())
}

fn print_source_includes(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let h_file = format!("\"{}\"", out.header_file);
    let source_includes = [h_file.as_str()];
    
    for inc in source_includes {
        writeln!(out.source, "#include {}", inc)?;
    }
    writeln!(out.source)?;
    Ok(())
}

fn print_typedefs(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let typedefs = [
        ("uint32_t", cc_type(&ValType::I32)),
        ("uint64_t", cc_type(&ValType::I64)),
        ("float", cc_type(&ValType::F32)),
        ("double", cc_type(&ValType::F64)),
    ];

    for td in typedefs {
        writeln!(out.header, "typedef {} {};", td.0, td.1)?;
    }
    writeln!(out.header)?;

    Ok(())
}

/* UTILITIES
 * - types -> string
 * - functions -> print
 */
fn cc_type(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        _ => unimplemented!(),
    }
}
