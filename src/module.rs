use crate::{OutputSpec, operators::ExpressionPrinter};
use anyhow::{Result, bail};
use convert_case::ccase;
use itertools::Itertools;
use std::{collections::HashMap, io::Write, mem};
use wasmparser::{
    Export, ExternalKind, FuncToValidate, FuncType, FuncValidator, FuncValidatorAllocations,
    FunctionBody, Import, Imports, KnownCustom, Name, Parser, Payload, TypeRef, ValType,
    ValidPayload, Validator, ValidatorResources, WasmModuleResources,
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

    // PARSE ONLY
    let (funcs, names) = read_module(wasm)?;

    // PRINT class declarations (header)
    print_module_declaration(&class_name, &funcs, &names, &mut output.header())?;

    // PRINT function definitions (body)
    print_source_includes(&mut output.body())?;
    print_function_definitions(&class_name, funcs, &names, &mut output.body())?;

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

// function name handling
// if given $name then $name, else generic f{index}
struct FuncNames<'a> {
    exports: HashMap<u32, Vec<&'a str>>, // func index -> export names
    names: HashMap<u32, &'a str>,        // func index -> name section $name
    locals: HashMap<(u32, u32), &'a str>, // func index, local index -> name section $name
}

// NAMING LOGIC
// every function is private, under w2c_{name}
// every export is a public wrapper, under the export name
// fn           export names    behavior
// <unnamed>    <none>          private w2c_f{index}()
// <unnamed>    a               private w2c_f{index}(), public wrapper a() { return w2c_f{index}() }
// $a           <none>          private w2c_a()
// $a           a, b            private w2c_a(), public wrappers a() b() { return w2c_a() }

// TODO: name collisions if $f0 matches generic format, or an export name starts with w2c_
// TODO: handle valid names in wat / wasm but invalid in c++
impl FuncNames<'_> {
    fn name(&self, index: u32) -> String {
        if let Some(name) = self.names.get(&index) {
            return name.to_string();
        }
        format!("{FUNC}{index}")
    }

    // list of export names of function
    fn exports(&self, index: u32) -> &[&str] {
        match self.exports.get(&index) {
            Some(exports) => exports,
            None => &[],
        }
    }

    // param and local names (both 'local')
    // name format "l{index}" unless explicitly given $name

    // TODO: more name collisions with generic l{index}, any future stack values s{n},
    // function names, etc
    fn local_name(&self, func: u32, local: u32) -> String {
        if let Some(name) = self.locals.get(&(func, local)) {
            return name.to_string();
        }
        // l0, l1 etc
        format!("{LOCAL}{local}")
    }
}

type Funcs<'a> = Vec<(FuncToValidate<ValidatorResources>, FunctionBody<'a>)>;

// PASS 1
// read-only pass to collect function bodies, function names, and local / param names only
fn read_module(wasm: &[u8]) -> Result<(Funcs<'_>, FuncNames<'_>)> {
    let mut validator = Validator::new();
    let mut funcs = Vec::new();
    let mut names = FuncNames {
        exports: HashMap::new(),
        names: HashMap::new(),
        locals: HashMap::new(),
    };
    let mut num_func_imports = 0;

    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload?;
        match validator.payload(&payload)? {
            ValidPayload::Func(f, body) => funcs.push((f, body)),
            ValidPayload::Ok => {
                match payload {
                    Payload::ExportSection(reader) => {
                        for export in reader {
                            let Export { name, kind, index } = export?;
                            match kind {
                                ExternalKind::Func if index < num_func_imports => {
                                    bail!("unimplemented: export of function import")
                                }
                                ExternalKind::Func => {
                                    names.exports.entry(index).or_default().push(name)
                                }
                                _ => bail!("unimplemented {kind:?}"),
                            }
                        }
                    }
                    Payload::ImportSection(reader) => {
                        for import in reader {
                            match import? {
                                Imports::Single(
                                    _,
                                    Import {
                                        ty: TypeRef::Func { .. },
                                        ..
                                    },
                                ) => num_func_imports += 1,
                                Imports::Single(_, _) => {} // TODO: other kinds of imports,
                                Imports::Compact1 { .. } | Imports::Compact2 { .. } => {
                                    bail!("unimplemented: compact imports")
                                }
                            }
                        }
                    }
                    Payload::CustomSection(reader) => {
                        // custom section > name section
                        if let KnownCustom::Name(reader) = reader.as_known() {
                            for subsection in reader {
                                match subsection? {
                                    // custom sect > name sect > function names
                                    Name::Function(map) => {
                                        for naming in map {
                                            let naming = naming?;
                                            names.names.insert(naming.index, naming.name);
                                        }
                                    }
                                    // custom sect > name sect > local names per function
                                    Name::Local(map) => {
                                        for func in map {
                                            let func = func?;
                                            for naming in func.names {
                                                let naming = naming?;
                                                names.locals.insert(
                                                    (func.index, naming.index),
                                                    naming.name,
                                                );
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            ValidPayload::Parser(_) => unimplemented!("component model"),
            ValidPayload::End(_) => break,
        }
    }

    Ok((funcs, names))
}

fn print_module_declaration(
    class_name: &str,
    funcs: &Funcs,
    names: &FuncNames,
    output: &mut impl Write,
) -> Result<()> {
    writeln!(output, "class {class_name} {{")?;

    // private functions signatures
    for (f, _body) in funcs {
        print_function_signature(f, &names.name(f.index), names, output)?;
        writeln!(output, ";")?;
    }

    writeln!(output, "\npublic:")?;

    // exports
    for (f, _body) in funcs {
        for export in names.exports(f.index) {
            print_wrapper(f, export, &names.name(f.index), names, output)?;
        }
    }

    writeln!(output, "}};")?;
    Ok(())
}

fn func_type(f: &FuncToValidate<ValidatorResources>) -> &FuncType {
    f.resources.sub_type_at(f.ty).unwrap().unwrap_func()
}

fn print_function_signature(
    f: &FuncToValidate<ValidatorResources>,
    name: &str,
    names: &FuncNames,
    output: &mut impl Write,
) -> Result<()> {
    let ty = func_type(f);
    match &ty.results() {
        [] => write!(output, "void")?,
        [vty] => write!(output, "{}", vty)?,
        vtys => write!(output, "std::tuple<{}>", vtys.iter().format(", "))?,
    }
    // function name
    write!(output, " {name}(")?;

    // params
    write!(
        output,
        "{}",
        ty.params()
            .iter()
            .enumerate()
            .format_with(", ", |(i, pty), fmt| {
                let local_index = i as u32;
                fmt(&format_args!(
                    "{pty} {}",
                    names.local_name(f.index, local_index)
                ))
            })
    )?;
    write!(output, ")")?;
    Ok(())
}

// i32 wrapper(i32 l0) { return name(l0); }
fn print_wrapper(
    f: &FuncToValidate<ValidatorResources>,
    wrapper_name: &str,
    name: &str,
    names: &FuncNames,
    output: &mut impl Write,
) -> Result<()> {
    print_function_signature(f, wrapper_name, names, output)?;

    // function name
    write!(output, " {{ return {name}(")?;

    // pass params
    writeln!(
        output,
        "{}); }}",
        func_type(f)
            .params()
            .iter()
            .enumerate()
            .format_with(", ", |(i, _pty), fmt| {
                let local_index = i as u32;
                fmt(&names.local_name(f.index, local_index))
            })
    )?;
    Ok(())
}

fn print_function_definitions(
    class_name: &str,
    funcs: Funcs,
    names: &FuncNames,
    output: &mut impl Write,
) -> Result<()> {
    let mut allocs = FuncValidatorAllocations::default();
    for (f, body) in funcs {
        let name = names.name(f.index);
        print_function_signature(&f, &format!("{class_name}::{name}"), names, output)?;
        writeln!(output, " {{")?;

        // functions
        let mut fval = f.into_validator(mem::take(&mut allocs));
        print_function_body(&mut fval, body, names, output)?;
        allocs = fval.into_allocations();

        writeln!(output, "}}")?;
    }
    Ok(())
}

fn print_function_body(
    f: &mut FuncValidator<ValidatorResources>,
    body: FunctionBody,
    names: &FuncNames,
    output: &mut impl Write,
) -> Result<()> {
    let mut reader = body.get_binary_reader();

    let num_params = f.len_locals();
    f.read_locals(&mut reader)?;

    // {} initializations to 0 (i32, i64) and 0.0 (f32, f64) (wasm defaults)
    // TODO: maybe relying on c++ defaults is a bit hacky, and a separate map
    // should hardcode wasm defaults

    // num_params to total locals = non-param locals
    for i in num_params..f.len_locals() {
        let ty = f.get_local_type(i).unwrap();
        writeln!(output, "{ty} {}{{}};", names.local_name(f.index(), i))?;
    }

    let mut printer = ExpressionPrinter::new(output);
    while !reader.eof() {
        reader.visit_operator(&mut printer.visitor(f.visitor(reader.original_position())))??;
    }
    reader.finish_expression(&printer.visitor(f.visitor(reader.original_position())))?;
    Ok(())
}
