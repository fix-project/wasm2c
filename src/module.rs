use crate::{OutputSpec, operators::ExpressionPrinter};
use anyhow::{Result, bail};
use bumpalo::Bump;
use convert_case::ccase;
use itertools::Itertools;
use std::{io::Write, mem};
use wasmparser::{
    Export, ExternalKind, FuncToValidate, FuncType, FuncValidator, FuncValidatorAllocations,
    FunctionBody, Parser, Payload, ValType, ValidPayload, Validator, ValidatorResources,
    WasmModuleResources,
};

/* STRING CONSTANTS */
static W2CC: &str = "w2cc_";
static FUNC: &str = "f";
static LOCAL: &str = "l";

pub fn wasm2c<U: Write>(name: &str, wasm: &[u8], output: &mut OutputSpec<U>) -> Result<String> {
    print_header_includes(&mut output.header())?;
    print_typedefs(&mut output.header())?;

    if let OutputSpec::Split { hh_include, cc, .. } = output {
        writeln!(cc, "#include \"{hh_include}\"")?;
    }

    let class_name = format!("{W2CC}{}", ccase!(snake, name));

    let funcs = print_module_declaration(&class_name, wasm, &mut output.header())?;

    print_source_includes(&mut output.body())?;
    print_function_definitions(&class_name, funcs, &mut output.body())?;

    Ok(class_name)
}

fn print_header_includes(out: &mut impl Write) -> Result<()> {
    writeln!(out, "#pragma once")?;
    writeln!(out)?;

    let header_includes = ["cstdint", "tuple"];
    for inc in header_includes {
        writeln!(out, "#include <{}>", inc)?;
    }
    writeln!(out)?;
    Ok(())
}

fn print_source_includes(out: &mut impl Write) -> Result<()> {
    let source_includes = [""; 0]; // no source includes yet
    for inc in source_includes {
        writeln!(out, "#include <{}>", inc)?;
    }
    writeln!(out)?;
    Ok(())
}

fn print_typedefs(out: &mut impl Write) -> Result<()> {
    let typedefs = [
        ("uint32_t", &ValType::I32),
        ("uint64_t", &ValType::I64),
        ("float", &ValType::F32),
        ("double", &ValType::F64),
    ];

    for td in typedefs {
        writeln!(out, "typedef {} {};", td.0, td.1)?;
    }
    writeln!(out)?;
    Ok(())
}

fn print_module_declaration<'a>(
    class_name: &str,
    wasm: &'a [u8],
    output: &mut impl Write,
) -> Result<Vec<(FuncToValidate<ValidatorResources>, FunctionBody<'a>)>> {
    writeln!(output, "class {class_name} {{")?;

    let mut validator = Validator::new();
    let export_names = Bump::new();
    let mut exports = Vec::new();
    let mut funcs = Vec::new();

    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload?;
        match validator.payload(&payload)? {
            ValidPayload::Func(f, body) => {
                // TODO: use func and param names from names section
                print_function_signature(&f, &format!("{FUNC}{}", f.index), output)?;
                writeln!(output, ";")?;
                funcs.push((f, body));
            }
            ValidPayload::Ok => {
                match payload {
                    Payload::ExportSection(reader) => {
                        for export in reader {
                            let export = export?;
                            let name = export_names.alloc_str(export.name);
                            exports.push(Export { name, ..export });
                        }
                    }
                    Payload::CustomSection(_) => {
                        // reader => name info
                    }
                    _ => {}
                }
            }
            ValidPayload::Parser(_) => unimplemented!("component model"),
            ValidPayload::End(_) => break,
        }
    }

    // print exports in public section of class
    writeln!(output, "\npublic:")?;

    for Export { name, kind, index } in exports {
        match kind {
            ExternalKind::Func => {
                let (f, _body) = &funcs[index as usize];
                print_function_signature(f, name, output)?;
                write!(output, " {{ return {FUNC}{}(", f.index)?; // TODO: may use name
                writeln!(
                    output,
                    "{}); }}",
                    func_type(f)
                        .params()
                        .iter()
                        .enumerate()
                        .format_with(", ", |(i, _pty), f| f(&format_args!("{LOCAL}{i}")))
                )?;
            }
            _ => bail!("unimplemented {kind:?}"),
        }
    }

    writeln!(output, "}};")?;

    Ok(funcs)
}

fn func_type(f: &FuncToValidate<ValidatorResources>) -> &FuncType {
    f.resources
        .sub_type_at(f.resources.type_index_of_function(f.ty).unwrap())
        .unwrap()
        .unwrap_func()
}

fn print_function_signature(
    f: &FuncToValidate<ValidatorResources>,
    name: &str,
    output: &mut impl Write,
) -> Result<()> {
    let ty = func_type(f);
    match &ty.results() {
        [] => write!(output, "void")?,
        [vty] => write!(output, "{}", vty)?,
        vtys => write!(output, "std::tuple<{}>", vtys.iter().format(", "))?,
    }
    write!(output, " {name}(")?;
    write!(
        output,
        "{}",
        ty.params()
            .iter()
            .enumerate()
            .format_with(", ", |(i, pty), f| f(&format_args!("{pty} {LOCAL}{i}")))
    )?;
    write!(output, ")")?;
    Ok(())
}

fn print_function_definitions(
    class_name: &str,
    funcs: Vec<(FuncToValidate<ValidatorResources>, FunctionBody)>,
    output: &mut impl Write,
) -> Result<()> {
    let mut allocs = FuncValidatorAllocations::default();
    for (f, body) in funcs {
        // TODO: use func and param names from names section
        print_function_signature(&f, &format!("{class_name}::{FUNC}{}", f.index), output)?;
        writeln!(output, " {{")?;
        let mut fval = f.into_validator(mem::take(&mut allocs));
        print_function_body(&mut fval, body, output)?;
        allocs = fval.into_allocations();
        writeln!(output, "}}")?;
    }
    Ok(())
}

fn print_function_body(
    f: &mut FuncValidator<ValidatorResources>,
    body: FunctionBody,
    output: &mut impl Write,
) -> Result<()> {
    let mut reader = body.get_binary_reader();
    f.read_locals(&mut reader)?;
    // TODO: print locals

    let mut printer = ExpressionPrinter::new(output);
    while !reader.eof() {
        reader.visit_operator(&mut printer.visitor(f.visitor(reader.original_position())))??;
    }
    reader.finish_expression(&printer.visitor(f.visitor(reader.original_position())))?;
    Ok(())
}
