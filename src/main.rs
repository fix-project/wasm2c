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
    ModuleArity, Operator, Parser, Payload, TypeRef, ValType, ValidPayload, Validator, 
    WasmModuleResources
};


fn main() -> Result<()> {

    // SECTION 1: I/O
    let mut input_stream = get_input()?;
    let mut output_stream = get_output_stream(&input_stream.dest_dir, &input_stream.name)?;

    // SECTION 1.5: FILENAME
    //

    // SECTION 2: BOILERPLATE
    print_includes(&mut output_stream)?;
    print_typedefs(&mut output_stream.header)?;
   
    // SECTION 3: printing program
    //
    // print_program(&mut input_stream, &mut output_stream)?;

    Ok(())
}

/* SECTION 1: I/O
 */
#[derive(clap::Parser)]
struct Args {
    src: Option<PathBuf>,

    #[arg(long)]
    dest: Option<PathBuf>,

    #[arg(long, required_unless_present = "src")]
    name: Option<String>,
}

struct Input<R: Read> {
    reader: BufReader<R>,
    dest_dir: PathBuf,
    name: String,
}

type Inputs = Input<Box<dyn Read>>;

struct Output<H: Write, C: Write> {
    header: BufWriter<H>,
    source: BufWriter<C>,
    header_name: String,
    source_name: String,
}

type Outputs = Output<Box<dyn Write>, Box<dyn Write>>;

/* SECTION 1.1: INPUT */
fn get_input() -> Result<Inputs> {
    let args = <Args as clap::Parser>::parse();

    let dest_dir = match args.dest {
        Some(dest) => { 
            if !dest.is_dir() {
                bail!("Expected directory, got file.");
            }
            dest
        },
        None => { current_dir()? },
    };

    match (args.src, args.name) {
        (Some(src), Some(name)) => {
            Ok( Input{ reader: get_input_stream(&src)?, dest_dir, name } )
        },
        (Some(src), None) => {
            Ok( Input{ reader: get_input_stream(&src)?, dest_dir, name: get_file_name(&src)? } )
        },
        (None, Some(name)) => {
            Ok( Input{ reader: BufReader::new_ringbuf(Box::new(stdin().lock()) as Box<dyn Read>), dest_dir, name } )
        },
        (None, None) => {
            bail!("If piping into stdin(), requires --name. Else, requires file (with optional --name)")
        }
    }
}

fn get_input_stream(path: &PathBuf) -> Result<BufReader<Box<dyn Read>>> {
    let f = File::open(path)?;
    Ok(BufReader::new_ringbuf(Box::new(f)))
}

fn get_file_name(path: &PathBuf) -> Result<String> {
    let path = Path::new(&path);
    if !path.is_file() {
        bail!("Not a file.");
    }
    
    let file_name = path.file_stem().ok_or_else(|| anyhow!("expected file name"))?.to_string_lossy();
    Ok(file_name.to_string())
}

/* SECTION 1.2: OUTPUT */

fn get_output_stream(dest_dir: &PathBuf, name: &String) -> Result<Outputs> {
    let header_name = format!("{}.h", &name);
    let source_name = format!("{}.cc", &name);
    let h = File::create(dest_dir.join(&header_name))?;
    let s = File::create(dest_dir.join(&source_name))?;

    let out = Output {
        header: BufWriter::new_ringbuf(Box::new(h) as Box<dyn Write>),
        source: BufWriter::new_ringbuf(Box::new(s) as Box<dyn Write>),
        header_name: header_name,
        source_name: source_name,
    };
    Ok(out)
}

/* SECTION 2: BOILERPLATE
 * includes and typedefs
 */
fn print_includes(out: &mut Outputs) -> Result<()> {
    print_header_includes(out)?;
    print_source_includes(out)?;
    Ok(())
}

fn print_header_includes(out: &mut Outputs) -> Result<()> {
    let header_includes = ["<stdint.h>"];
    for inc in header_includes {
        writeln!(out.header, "#include {}", inc)?;
    }
    writeln!(out.header)?;
    Ok(())
}

fn print_source_includes(out: &mut Outputs) -> Result<()> {
    let h_file = format!("\"{}\"", out.header_name);
    let source_includes = [h_file.as_str()];
    
    for inc in source_includes {
        writeln!(out.source, "#include {}", inc)?;
    }
    writeln!(out.source)?;
    Ok(())
}

fn print_typedefs(out: &mut impl Write) -> Result<()> {
    let typedefs = [
        ("uint32_t", cc_type(&ValType::I32)),
        ("uint64_t", cc_type(&ValType::I64)),
        ("float", cc_type(&ValType::F32)),
        ("double", cc_type(&ValType::F64)),
    ];

    for td in typedefs {
        writeln!(out, "typedef {} {};", td.0, td.1)?;
    }
    writeln!(out)?;

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
