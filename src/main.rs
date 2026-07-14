// CURRENTLY SUPPORTS:
// TYPES: i32, i64, f32, f64 -> uint32_t, uint64_t, float, double
// SINGLE RETURN VALUES
// EXPORT: functions ONLY

use color_eyre::eyre::{Result, bail};
use buffer_redux::{BufReader, BufWriter};
use std::{io::{BufRead, Read, Write, stdin, stdout}, fs::File};
use wasmparser::{
    Chunk, Export, ExternalKind, FuncToValidate, FuncType, FuncValidatorAllocations, FunctionBody, 
    ModuleArity, Operator, Parser, Payload, TypeRef, ValType, ValidPayload, Validator, 
    WasmModuleResources
};

static PUBLIC_STR: &'static str = "public:";
static PRIVATE_STR: &'static str = "private:";

#[derive(PartialEq)]
enum CodeSection {
    Public,
    Private,
    None,
}

struct Output<H: Write, C: Write> {
    header: H,
    source: C,
    header_name: String,
    source_name: String,
}

fn get_input_stream() -> BufReader<impl Read> {
    BufReader::new_ringbuf(stdin())
}

type FileOutput = Output<BufWriter<File>, BufWriter<File>>;

fn get_output_stream() -> Result<FileOutput> {
    let h = File::create("out.h")?;
    let s = File::create("out.cpp")?;
    let out = Output {
        header: BufWriter::new_ringbuf(h),
        source: BufWriter::new_ringbuf(s),
        header_name: "out.h".to_string(),
        source_name: "out.cpp".to_string(),
    };
    Ok(out)
}

fn compile_from_stream(
    input_stream: &mut BufReader<impl Read>,
    output_stream: &mut FileOutput,
) -> Result<()> {
    let mut parser = Parser::new(0);
    let mut eof = false;
    let mut section = CodeSection::None;

    // VALIDATOR stuff
    let mut validator = Validator::new();
    let mut allocs = FuncValidatorAllocations::default();

    loop {
        // PER PAYLOAD
        match parser.parse(input_stream.buffer(), eof)? {
            Chunk::Parsed { consumed, payload } => {
                match validator.payload(&payload)? {
                    ValidPayload::Func(f, body) => {
                        dbg!("------- FUNC -------");
                        if section != CodeSection::Private {
                            writeln!(output_stream.header, "\n{}", PRIVATE_STR)?;
                            section = CodeSection::Private;
                        }

                        allocs = code_function(output_stream, f, body, allocs)?;
                        // allocs = handle(f, body, allocs)?,
                    }, 
                    ValidPayload::Parser(_) => unimplemented!("component model"),
                    ValidPayload::End(_) => break,
                    ValidPayload::Ok => { 
                        match payload {
                            // Payload::ImportSection(reader) => {
                            //     for import in reader.into_imports().flatten() {
                            //         dbg!(import); // TODO
                            //     }
                            // },
                            Payload::ExportSection(reader) => {
                                if section != CodeSection::Public {
                                    writeln!(output_stream.header, "\n{}", PUBLIC_STR)?;
                                    section = CodeSection::Public;
                                }

                                for export in reader.into_iter().flatten() {
                                    code_export(output_stream, export, &validator)?;
                                }
                            },  
                            Payload::CustomSection(_) => {
                                dbg!("------- CUSTOM -------");
                            },
                            _ => {},
                        }
                    },
                }
                input_stream.consume(consumed);
            }
            Chunk::NeedMoreData(hint) => {
                if eof {
                    bail!("unexpected end");
                }
                input_stream.reserve(hint as usize);
                if input_stream.read_into_buf()? == 0 {
                    eof = true;
                }
            }
        }
    }

    Ok(())
}

fn code_export(out: &mut FileOutput, export: Export, validator: &Validator) -> Result<()> {
    // TODO:
    let types = validator.types(0).unwrap();
    match export.kind {
        ExternalKind::Func => {
            let export_type = types[types.core_function_at(export.index)].unwrap_func();
            let params = export_type.params();
            let results = export_type.results();

            print_fn_prototype(&mut out.header, export.name, params, results)?;

            // SOURCE FILE SHENANIGANS (just call the respective function)
            print_fn_signature(&mut out.source, export.name, params, results)?;
            writeln!(out.source, "return f{}();", export.index)?;
            writeln!(out.source, "}};\n")?;
        },
        _ => {}, // TODO: other exports worry about later
    }
    Ok(())
}

fn print_fn_prototype(
    out: &mut impl Write,
    name: &str,
    params: &[ValType],
    results: &[ValType],
) -> Result<()> {
    let cc_return = 
        if results.is_empty() {
            "void"
        } else {
            cc_type(&results[0]) // TODO multiple values
        };

    let mut cc_params = String::new();
    for (i, ty) in params.iter().enumerate() {
        if i > 0 {
            cc_params += ", ";
        }
        cc_params += cc_type(ty);
    }

    // <return> f0(<param>, <param>);
    writeln!(out, "{cc_return} {name}({cc_params});")?;

    Ok(())
}

fn print_fn_signature(
    out: &mut impl Write,
    name: &str,
    params: &[ValType],
    results: &[ValType]
) -> Result<()> {
    let cc_return = 
        if results.is_empty() {
            "void"
        } else {
            cc_type(&results[0]) // TODO multiple values
        };

    let mut cc_params = String::new();
    for (i, ty) in params.iter().enumerate() {
        if i > 0 {
            cc_params += ", ";
        }
        cc_params += cc_type(ty);
    }

    // <return> f0(<param>, <param>);
    writeln!(out, "{cc_return} Module::{name}({cc_params}) {{")?;

    Ok(())
}

fn code_function<T: WasmModuleResources>(
    out: &mut FileOutput,
    f: FuncToValidate<T>,
    body: FunctionBody<'_>,
    allocs: FuncValidatorAllocations,
) -> Result<FuncValidatorAllocations> {
    let mut func_validator = f.into_validator(allocs);

    // --- PRINT FUNCTION SIGNATURE ---
    let func_id = func_validator.index(); // function id

    let func_type_id = func_validator.type_index_of_function(func_id).unwrap();
    let func_type = func_validator.sub_type_at(func_type_id).unwrap().unwrap_func(); // function type
    let results = func_type.results();
    let params = func_type.params();

    let name = format!("f{func_id}");
    print_fn_prototype(&mut out.header, &name, params, results)?;

    // --- PRINT ACTUAL FUNCTION CONTENTS ---
    // f: function validator
    // - locals
    // - 
    print_fn_signature(&mut out.source, &name, params, results)?;
    // let locals_reader = body.get_locals_reader()?;
    // let mut ops_reader = body.get_binary_reader_for_operators()?;
    
    let mut reader = body.get_binary_reader();
    func_validator.read_locals(&mut reader)?;

    
    while !reader.eof() {
        // for depth in 0..func_validator.operand_stack_height() {
        //     print!(" {:?}", func_validator.get_operand_type(depth as usize));
        // }
        // println!();

        // let op = reader.visit_operator(&mut func_validator.visitor(reader.original_position()))??;
        // dbg!(op);
        //

        let op = reader.peek_operator(&func_validator.visitor(reader.original_position()))?;

        // consume the op
        reader.visit_operator(&mut func_validator.visitor(reader.original_position()))??;

        // STACK -> VARS
        let n_stack = func_validator.operand_stack_height();
        match op {
            Operator::I32Const { value } => {
                let n_var = n_stack - 1;
                writeln!(out.source, "{} var_i{} = {};", cc_type(&ValType::I32), n_var, value)?;
            },
            _ => {},
        }

    }
    reader.finish_expression(&func_validator.visitor(reader.original_position()))?;

    writeln!(out.source, "return var_i0;")?;
    writeln!(out.source, "}};\n")?;

    Ok(func_validator.into_allocations())
}

fn print_includes(out: &mut FileOutput) -> Result<()> {
    let header_includes = ["<stdint.h>"];
    let header_file = format!("\"{}\"", &out.header_name);
    let source_includes = [header_file.as_str()];

    // HEADER INCLUDES
    for inc in header_includes {
        writeln!(out.header, "#include {}", inc)?;
    }
    writeln!(out.header)?;

    // HEADER INCLUDES
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

fn print_program(
    input_stream: &mut BufReader<impl Read>,
    output_stream: &mut FileOutput,
) -> Result<()> {
    writeln!(output_stream.header, "class Module {{")?;
    compile_from_stream(input_stream, output_stream)?;
    writeln!(output_stream.header, "}};")?;

    output_stream.header.flush()?;
    output_stream.source.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    // Step 1: parse input, and use Validator on every payload
    color_eyre::install()?; 

    // PARSER stuff
    let mut input_stream = get_input_stream();
    let mut output_stream: FileOutput = get_output_stream()?;

    // TYPEDEF stuff
    print_includes(&mut output_stream)?;
    print_typedefs(&mut output_stream.header)?;
    
    print_program(&mut input_stream, &mut output_stream)?;

    Ok(())
}

// function to validate(?)
// fn handle<T: WasmModuleResources>(
//     f: FuncToValidate<T>,
//     body: FunctionBody<'_>,
//     allocs: FuncValidatorAllocations,
// ) -> Result<FuncValidatorAllocations> {
//     // Step 2: go operator-by-operator and validate each function
//     let mut func_validator = f.into_validator(allocs);
//     let mut reader = body.get_binary_reader();
//     let mut operator_count = 0;
//     func_validator.read_locals(&mut reader)?;
//     while !reader.eof() {
//         print!("Before operator {operator_count}, here are the operands on the stack:");
//         for depth in 0..func_validator.operand_stack_height() {
//             print!(" {:?}", func_validator.get_operand_type(depth as usize));
//         }
//         println!();
//
//         reader.visit_operator(&mut func_validator.visitor(reader.original_position()))??;
//         operator_count += 1;
//     }
//     reader.finish_expression(&func_validator.visitor(reader.original_position()))?;
//     Ok(func_validator.into_allocations())
// }
//
// code generation

fn cc_type(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        _ => unimplemented!(),
    }
}

