use anyhow::{bail, anyhow, Result};
use buffer_redux::{BufReader, BufWriter};
use std::io::{BufRead, Read, Write};
use std::fs::File;
use wasmparser::{
    Chunk, ExternalKind, FuncValidator, FuncValidatorAllocations, FunctionBody, ModuleArity, Operator, Parser, Payload, SubType, ValType, ValidPayload, Validator, ValidatorResources, WasmModuleResources
};

use clap::Parser as ClapParser;
use clio::*;
use convert_case::ccase;


/* STRING CONSTANTS */
static W2CC: &'static str = "w2cc_";
static FUNC: &'static str = "fn_";
static LOCAL: &'static str = "l";
static DEFAULT_VALUE: &'static str = "0";
static PUBLIC_SECT: &'static str = "\npublic:";
static PRIVATE_SECT: &'static str = "\nprivate:";

fn main() -> Result<()> {

    // SECTION 1: I/O
    let mut config = get_config()?;
    let mut output = get_output(&mut config)?;

    // SECTION 2: BOILERPLATE
    print_includes(&mut output)?;
    print_typedefs(&mut output)?;
   
    // SECTION 3: printing program
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
    let header_file = format!("{}.hh", config.name);
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

/* SECTION 3.1: PROGRAM-CONTENTS */
fn print_program(input: &mut Config<impl Read>, output: &mut Output<impl Write, impl Write>)
-> Result<()>{
    writeln!(output.header, "class {} {{", ccase!(pascal, &input.name))?;
    print_class(input, output)?;
    writeln!(output.header, "}};")?;

    Ok(())
}

struct ExportCopy {
    name: String,
    kind: ExternalKind,
    index: u32,
}

/* SECTION 3.2: CLASS CONTENTS */
fn print_class(input: &mut Config<impl Read>, output: &mut Output<impl Write, impl Write>) -> Result<()> {

    let mut parser = Parser::new(0);
    let mut eof = false;

    let mut validator = Validator::new();
    let mut allocs = FuncValidatorAllocations::default();

    let mut func_metadata: Vec<FuncValidator<ValidatorResources>> = Vec::new();
    let mut exports: Vec<ExportCopy> = Vec::new();

    loop {
        let buf = input.reader.buffer();
        match parser.parse(buf, eof)? {
            Chunk::Parsed { consumed, payload } => {
                match validator.payload(&payload)? {
                    ValidPayload::Func(f, body) => {
                        let func_validator = f.into_validator(allocs);
                        func_metadata.push(func_validator.clone()); // save FuncToValidate for fn signatures later
                        allocs = print_function(func_validator, body, output)?;
                    },
                    ValidPayload::Ok => {
                        match payload {
                            Payload::ExportSection(reader) => {
                                for export in reader.into_iter().flatten() {
                                    let copy = ExportCopy{
                                        name: export.name.to_owned(),
                                        kind: export.kind,
                                        index: export.index,
                                    };
                                    exports.push(copy);
                                }
                                // reader => save export info
                            },
                            Payload::CustomSection(_) => {
                                // reader => name info
                            },
                            _ => {},
                        }
                    },
                    ValidPayload::Parser(_) => unimplemented!("component model"),
                    ValidPayload::End(_) => {
                        break
                    }
                }
                input.reader.consume(consumed);
            },
            Chunk::NeedMoreData(hint) => {
                if eof { bail!("unexpected end"); }
                input.reader.reserve(hint as usize);

                // last chance to do anything with validator
                if input.reader.read_into_buf()? == 0 { 

                    print_exports(&exports, &validator, output)?;
                    eof = true; 
                }
            },
        }
    }


    Ok(())
}

/* SECTION 3.3: FUNCTION CONTENTS */
fn print_function<T: WasmModuleResources> (
    mut f: FuncValidator<T>, 
    body: FunctionBody, 
    output: &mut Output<impl Write, impl Write>
)-> Result<FuncValidatorAllocations> {
    let func_type = get_function_type(&f);
    let mut reader = body.get_binary_reader();

    // FUNCTION SIGNATURE (HEADER)
    writeln!(output.header, "{};", get_function_signature(func_type, &LabelType::Index(f.index()), false, false))?;

    // FUNCTION SIGNATURE (SOURCE)
    writeln!(output.source, "{} {{", get_function_signature(func_type, &LabelType::Index(f.index()), true, false))?;
    
    // ---- LOCALS ---- //
    f.read_locals(&mut reader)?;
    print_locals(&f, output)?;
    
    // ---- OPERANDS ---- //
    print_operands(&mut f, body, output)?;

    writeln!(output.source, "}}\n")?;
    
    Ok(f.into_allocations())
}

fn print_locals<T: WasmModuleResources> (f: &FuncValidator<T>, output: &mut Output<impl Write, impl Write>) -> Result<()> {
    let n_params = get_function_type(&f).0.len() as u32;
    for i in n_params..f.len_locals() {
        let local_type = f.get_local_type(i).unwrap();
        let local_str = format!("{} {}{} = {};", cc_type(&local_type), LOCAL, i, DEFAULT_VALUE);
        writeln!(output.source, "{}", local_str)?;
    }
    Ok(())
}

fn subtype_to_function_type(subtype: &SubType) -> (&[ValType], &[ValType]) {
    let func_type = subtype.unwrap_func();
    (func_type.params(), func_type.results())
}

fn get_function_type<T: WasmModuleResources> (f: &FuncValidator<T>) -> (&[ValType], &[ValType]) {
    let func_type_index = f.type_index_of_function(f.index()).unwrap();
    let func_subtype = f.sub_type_at(func_type_index).unwrap();
    subtype_to_function_type(func_subtype)
}

enum LabelType {
    Name(String),
    Index(u32),
}

fn get_function_signature(
    func_type: (&[ValType], &[ValType]), 
    label: &LabelType, 
    has_param_name: bool,
    call: bool,
) -> String {
    // PARAMS
    let params = func_type.0;
    let mut params_str = String::new();

    for i in 0..params.len() {
        if i > 0 { 
            params_str += ", "; 
        }
        let ty = params[i as usize];
        params_str += cc_type(&ty);
        if has_param_name {
            params_str += &format!(" {LOCAL}{i}");
        }
    }

    // RETURN
    let mut res_tuple = String::from("std::tuple<");
    let results_str = 
        if call { "return" }
        else {
            let results = func_type.1;
            match results.len() {
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
            }
        };

    // NAME
    let label_str = match label {
        LabelType::Name(name) => {
            name
        },
        LabelType::Index(u32) => {
            &format!("{W2CC}{FUNC}{u32}")
        },
    };

    format!("{} {}({})", results_str, label_str, params_str)
}

/* SECTION 3.4: OPERANDS & FUNCTION LOGIC */
struct TypeStackData {
    next_idx: i32,
    max_idx: i32
}

impl Default for TypeStackData {
    fn default() -> Self {
        TypeStackData { next_idx: 0, max_idx: -1 }
    }
}

#[derive(Default)]
struct TypeStack {
    i32_ht: TypeStackData,
    i64_ht: TypeStackData,
    f32_ht: TypeStackData,
    f64_ht: TypeStackData,
}

impl TypeStack {
    fn slot(&mut self, ty: ValType) -> &mut TypeStackData {
        match ty {
            ValType::I32 => &mut self.i32_ht,
            ValType::I64 => &mut self.i64_ht,
            ValType::F32 => &mut self.f32_ht,
            ValType::F64 => &mut self.f64_ht,
            _ => unimplemented!(),
        }
    }

    fn inc(&mut self, ty: ValType) { 
        let data = self.slot(ty);
        if data.next_idx > data.max_idx {
            data.max_idx = data.next_idx
        }
        data.next_idx += 1;
    }
    fn dec(&mut self, ty: ValType) { self.slot(ty).next_idx -= 1 }
    fn get(&mut self, ty: ValType) -> i32 { self.slot(ty).next_idx }

    // indicates if new declared variable of type ty is needed
    fn declared(&mut self, ty: ValType) -> bool {
        let data = self.slot(ty);
        data.next_idx <= data.max_idx

    }
}

fn print_operands<T: WasmModuleResources> (
    f: &mut FuncValidator<T>, 
    body: FunctionBody, 
    output: &mut Output<impl Write, impl Write>
) -> Result<()> {
    f.validate(&body)?;

    let mut stack = TypeStack::default();
    let mut ops = body.get_operators_reader()?;
    while !ops.eof() {
        let op = ops.read()?; 
        match op {
            // GET
            // SET
            // CONST
            Operator::I32Const { value } => {
                print_i32const(value, &mut stack, output)?;
            },
            Operator::I64Const { .. } => {
                // print_i64const(output, value);
            },
            Operator::F32Const { .. } => {
                // print_f32const(output, value);
            },
            Operator::F64Const { .. } => {
                // print_f64const(output, value);
            },
            // ADD
            // SUB
            // MUL
            // DIVS
            // DIVU
            _ => { },
        }
    }

    // RETURN
    print_return(f, &mut stack, output)?;

    Ok(())
}

fn print_return<T: WasmModuleResources>(f: &mut FuncValidator<T>, 
    stack: &mut TypeStack,
    output: &mut Output<impl Write, impl Write>
) -> Result<()> {
    let func_type = get_function_type(&f);
    let results = func_type.1;

    match results.len() {
        0 => {
            writeln!(output.source, "return;")?;
        }
        1 => {
            let ty = results[0];
            let name = var_name(&ty, stack.get(ty) - 1);
            stack.dec(ty);

            writeln!(output.source, "return {name};")?;
        }
        _ => { 
            let mut ret_str = String::from("return {");
            for i in 0..results.len() {
                if i > 0 {
                    ret_str += ", ";
                }
                let ty = results[i];
                ret_str += &var_name(&ty, stack.get(ty) - 1);

                stack.dec(ty);
            }
            ret_str += "};";

            writeln!(output.source, "{ret_str}")?;
        }

    }
    Ok(())
}


// operand codegen
fn var_name(ty: &ValType, index: i32) -> String {
    format!("{}_{}", cc_type(&ty), index)
}

// [] -> [i32]
fn print_i32const(value: i32, stack: &mut TypeStack, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let index = stack.get(ValType::I32);
    let ty = cc_type(&ValType::I32);
    let var_name = var_name(&ValType::I32, index);

    if stack.declared(ValType::I32) {
        writeln!(out.source, "{} = {};", var_name, value)?;
    } else {
        writeln!(out.source, "{} {} = {};", ty, var_name, value)?;
    }
    stack.inc(ValType::I32);

    Ok(())
}

/* EXPORTS */
fn print_exports(
    exports: &[ExportCopy], 
    validator: &Validator,
    output: &mut Output<impl Write, impl Write>
) -> Result<()> {
    writeln!(output.header, "{}", PUBLIC_SECT)?;
    let types = validator.types(0).unwrap();

    for e in exports {
        match e.kind {
            ExternalKind::Func => {
                let type_id = types.core_function_at(e.index);
                let subtype = &types[type_id];
                let func_type = subtype_to_function_type(subtype);

                let label = LabelType::Name(e.name.clone());
                
                // header
                let sgn_header = get_function_signature(func_type, &label, false, false);
                writeln!(output.header, "{};", sgn_header)?;

                // source
                let sgn_source = get_function_signature(func_type, &label, true, false);
                let call = get_function_signature(func_type, &LabelType::Index(e.index), true, true);
                writeln!(output.source, "{} {{", sgn_source)?;
                writeln!(output.source, "{};", call)?;
                writeln!(output.source, "}}")?;

            }
            _ => {  }
        };
    }
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

