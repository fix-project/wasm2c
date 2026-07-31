
// SINGLE RETURN VALUES
// EXPORT: functions only

use anyhow::{bail, anyhow, Result};
use buffer_redux::{BufReader, BufWriter};
use std::io::{BufRead, Read, Write};
use std::fs::File;
use wasmparser::{
    Chunk, Export, ExternalKind, FuncToValidate, FuncType, FuncValidator, FuncValidatorAllocations, FunctionBody, ModuleArity, Operator, Parser, Payload, TypeRef, ValType, ValidPayload, Validator, ValidatorResources, WasmModuleResources
};

use clap::Parser as ClapParser;
use clio::*;
use convert_case::ccase;

/* STRING CONSTANTS */
static W2CC: &'static str = "w2cc_";
static FUNC: &'static str = "fn_";

fn main() -> Result<()> {

    // SECTION 1: I/O
    let mut config = get_config()?;
    let mut output = get_output(&mut config)?;

    // SECTION 2: BOILERPLATE
    print_includes(&mut output)?;
    print_typedefs(&mut output)?;
   
    // SECTION 3: printing program
    //
    print_program(&mut config, &mut output)?;

    Ok(())
}

/* SECTION 1: I/O
 */
#[derive(ClapParser)]
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
    let source_includes = [h_file.as_str(), "<tuple>"];
    
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


/* SECTION 3: MAIN PROGRAM LOOP
 * - functions: immediate generation
 * - exports: save iterators
 * - names: last
 */
fn print_program(input: &mut Config<impl Read>, output: &mut Output<impl Write, impl Write>)
-> Result<()>{
    writeln!(output.source, "class {} {{", ccase!(pascal, &input.name))?;
    print_class(input, output)?;
    writeln!(output.source, "}};")?;

    Ok(())
}

fn print_class(input: &mut Config<impl Read>, output: &mut Output<impl Write, impl Write>) -> Result<()> {

    let mut parser = Parser::new(0);
    let mut eof = false;

    let mut validator = Validator::new();
    let mut allocs = FuncValidatorAllocations::default();

    let mut func_metadata: Vec<FuncValidator<ValidatorResources>> = Vec::new();

    loop {
        let buf = input.reader.buffer().iter().cloned().collect::<Vec<_>>();
        match parser.parse(&buf, eof)? {
            Chunk::Parsed { consumed, payload } => {
                match validator.payload(&payload)? {
                    ValidPayload::Func(f, body) => {
                        let mut func_validator = f.into_validator(allocs);
                        func_metadata.push(func_validator.clone()); // save FuncToValidate for fn signatures later
                        allocs = print_function(func_validator, body, input, output)?;
                    },
                    ValidPayload::Ok => {
                        match payload {
                            Payload::ExportSection(reader) => {
                                // reader => save export info
                            },
                            Payload::CustomSection(reader) => {
                                // reader => name info
                            },
                            _ => {},
                        }
                    },
                    ValidPayload::Parser(_) => unimplemented!("component model"),
                    ValidPayload::End(_) => break
                }
                input.reader.consume(consumed);
            },
            Chunk::NeedMoreData(hint) => {
                if eof { bail!("unexpected end"); }
                input.reader.reserve(hint as usize);
                if input.reader.read_into_buf()? == 0 { eof = true; }
            },
        }
    }
    Ok(())
}

fn print_function<T: WasmModuleResources> (
    f: FuncValidator<T>, 
    body: FunctionBody, 
    input: &mut Config<impl Read>, output: &mut Output<impl Write, impl Write>
)-> Result<FuncValidatorAllocations> {
    
    dbg!(f.len_locals());
    for i in 0..f.len_locals() {
        dbg!(f.get_local_type(i));
    } 
    // <return> w2cc_function_0(<param type> param_0, <type> param_1, ...) {
    let func_type = get_function_type(&f);
    writeln!(output.source, "{}", get_function_signature(func_type, LabelType::Index(f.index()), true))?;
    
    Ok(f.into_allocations())
}

fn get_function_type<T: WasmModuleResources> (f: &FuncValidator<T>) -> (&[ValType], &[ValType]) {
    let func_type_index = f.type_index_of_function(f.index()).unwrap();
    let func_type = f.sub_type_at(func_type_index).unwrap().unwrap_func();
    (func_type.params(), func_type.results())
}

enum LabelType {
    Name(String),
    Index(u32),
}
fn get_function_signature(func_type: (&[ValType], &[ValType]), label: LabelType, has_param_name: bool) -> String {
    // PARAMS
    let params = func_type.0;
    let mut params_str = String::new();
    for (i, ty) in params.iter().enumerate() {
        if i > 0 { 
            params_str += ", "; 
        }
        params_str += cc_type(ty);
        if has_param_name {
            params_str += &format!(" p{i}");
        }
    }

    // RETURN
    let results = func_type.1;
    let mut res_tuple = String::from("std::tuple<");
    let mut results_str = 
        match func_type.1.len() {
            0 => "void",
            1 => cc_type(&results[0]),
            _ => {
                for (i, ty) in results.iter().enumerate() {
                    if i > 0 {
                        res_tuple += ", "; 
                    }
                    res_tuple += cc_type(ty);
                }
                res_tuple += ">";
                &res_tuple
            },
        };

    // NAME
    let label_str = match label {
        LabelType::Name(name) => {
            name
        },
        LabelType::Index(u32) => {
            format!("{W2CC}{FUNC}{u32}")
        },
    };

    format!("{} {}({})", results_str, label_str, params_str)
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

fn function_sgn() -> String {
    todo!()
}

