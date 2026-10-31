use crate::{OutputSpec, operators::ExpressionPrinter};
use anyhow::{Result, bail};
use convert_case::ccase;
use itertools::Itertools;
use std::{collections::HashMap, io::Write, mem};
use wasmparser::{
    Export, ExternalKind, FuncToValidate, FuncType, FuncValidatorAllocations, FunctionBody, Import,
    Imports, KnownCustom, Name, Parser, Payload, TypeRef, ValType, ValidPayload, Validator,
    ValidatorResources, WasmModuleResources,
};

/* STRING CONSTANTS */
static W2CC: &str = "w2cc_";

pub fn wasm2c<U: Write>(name: &str, wasm: &[u8], output: &mut OutputSpec<U>) -> Result<String> {
    print_header_includes(&mut output.header())?;
    print_typedefs(&mut output.header())?;

    if let OutputSpec::Split { hh_include, cc, .. } = output {
        writeln!(cc, "#include \"{hh_include}\"")?;
    }

    let class_name = format!("{W2CC}{}", ccase!(snake, name));

    let module = Module::new(wasm)?;

    // PRINT class declarations (header)
    module.print_declaration(&class_name, &mut output.header())?;

    // PRINT function definitions (body)
    print_source_includes(&mut output.body())?;
    module.print_function_definitions(&class_name, &mut output.body())?;

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

// TODO: name collisions (need gensym, currently issue #23)
// TODO: handle valid names in wat / wasm but invalid in c++ (need mangling, currently issue #22)

// stolen from wasmprinter
struct NamingMap<K>(HashMap<u32, String>, std::marker::PhantomData<K>);

impl<K> Default for NamingMap<K> {
    fn default() -> Self {
        NamingMap(HashMap::new(), std::marker::PhantomData)
    }
}

impl<K: NamingNamespace> NamingMap<K> {
    fn get(&self, key: u32) -> String {
        if let Some(name) = self.0.get(&key) {
            return name.clone();
        }
        format!("{}{key}", K::desc())
    }
}

/// Helper trait for the `NamingMap` type's `K` type parameter.
trait NamingNamespace {
    fn desc() -> &'static str;
}

macro_rules! naming_namespaces {
    ($(struct $name:ident => $desc:tt)*) => ($(
        struct $name;

        impl NamingNamespace for $name {
            fn desc() -> &'static str { $desc }
        }
    )*)
}

naming_namespaces! {
    struct NameFunc => "func"
    struct NameLocal => "local"
}

struct Module<'a> {
    exports: Vec<Export<'a>>,
    funcs: Vec<Func<'a>>,
    func_names: NamingMap<NameFunc>,
    num_func_imports: usize,
}

struct Func<'a> {
    func: FuncToValidate<ValidatorResources>,
    body: FunctionBody<'a>,
    local_names: NamingMap<NameLocal>,
}

impl<'a> Module<'a> {
    // pass to collect function bodies, function names, and local / param names only
    fn new(wasm: &'a [u8]) -> Result<Self> {
        let mut validator = Validator::new();

        let mut exports = Vec::new();
        let mut funcs = Vec::new();
        let mut func_names = NamingMap::default();
        let mut num_func_imports = 0;

        for payload in Parser::new(0).parse_all(wasm) {
            let payload = payload?;
            match validator.payload(&payload)? {
                ValidPayload::Func(func, body) => funcs.push(Func {
                    func,
                    body,
                    local_names: Default::default(),
                }),
                ValidPayload::Ok => {
                    match payload {
                        Payload::ExportSection(reader) => {
                            for export in reader {
                                let export = export?;
                                match export.kind {
                                    ExternalKind::Func => exports.push(export),
                                    _ => bail!("unimplemented {:?}", export.kind),
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
                                    ) => num_func_imports += 1, // TODO: actually use import,
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
                                                func_names
                                                    .0
                                                    .insert(naming.index, naming.name.to_string());
                                            }
                                        }
                                        // custom sect > name sect > local names per function
                                        Name::Local(map) => {
                                            for func in map {
                                                let func = func?;
                                                for naming in func.names {
                                                    let naming = naming?;
                                                    funcs
                                                        .get_mut(func.index as usize)
                                                        .expect("func index in range")
                                                        .local_names
                                                        .0
                                                        .insert(
                                                            naming.index,
                                                            naming.name.to_string(),
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

        Ok(Module {
            exports,
            funcs,
            func_names,
            num_func_imports,
        })
    }

    fn print_declaration(&self, class_name: &str, output: &mut impl Write) -> Result<()> {
        writeln!(output, "class {class_name} {{")?;

        // private functions signatures
        for (i, f) in self.funcs.iter().enumerate() {
            f.print_signature(&self.func_names.get(i as u32), output)?;
            writeln!(output, ";")?;
        }

        writeln!(output, "\npublic:")?;

        // exports
        for Export { name, kind, index } in &self.exports {
            if kind != &ExternalKind::Func {
                unimplemented!("only support func exports right now");
            }
            if (*index as usize) < self.num_func_imports {
                unimplemented!("export of func import");
            }
            let f = self
                .funcs
                .get(*index as usize - self.num_func_imports)
                .expect("valid export");

            f.print_signature(name, output)?;
            // function name
            write!(output, " {{ return {}(", self.func_names.get(*index))?;

            // pass params
            writeln!(
                output,
                "{}); }}",
                (0..f.ty().params().len())
                    .format_with(", ", |i, fmt| { fmt(&f.local_names.get(i as u32)) })
            )?;
        }

        writeln!(output, "}};")?;
        Ok(())
    }

    fn print_function_definitions(self, class_name: &str, output: &mut impl Write) -> Result<()> {
        let mut allocs = FuncValidatorAllocations::default();
        for (i, f) in self.funcs.into_iter().enumerate() {
            let name = self.func_names.get(i as u32);
            f.print_signature(&format!("{class_name}::{name}"), output)?;
            writeln!(output, " {{")?;

            allocs = f.print_body(mem::take(&mut allocs), output)?;

            writeln!(output, "}}")?;
        }
        Ok(())
    }
}

impl<'a> Func<'a> {
    fn print_signature(&self, name: &str, output: &mut impl Write) -> Result<()> {
        let ty = self.ty();
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
                    fmt(&format_args!("{pty} {}", self.local_names.get(i as u32)))
                })
        )?;
        write!(output, ")")?;
        Ok(())
    }

    fn print_body(
        self,
        allocs: FuncValidatorAllocations,
        output: &mut impl Write,
    ) -> Result<FuncValidatorAllocations> {
        let Func {
            func,
            body,
            local_names,
        } = self;
        let mut reader = body.get_binary_reader();
        let mut f = func.into_validator(allocs);

        let num_params = f.len_locals();
        f.read_locals(&mut reader)?;

        // {} initializations to 0 (i32, i64) and 0.0 (f32, f64) (wasm defaults)
        // TODO: maybe relying on c++ defaults is a bit hacky, and a separate map
        // should hardcode wasm defaults

        // num_params to total locals = non-param locals
        for i in num_params..f.len_locals() {
            let ty = f.get_local_type(i).unwrap();
            writeln!(output, "{ty} {}{{}};", local_names.get(i))?;
        }

        let mut printer = ExpressionPrinter::new(output);
        let mut reader = body.get_operators_reader()?;
        while !reader.eof() {
            let (op, offset) = reader.read_with_offset()?;
            printer.op(&mut f, &op, offset)?;
        }
        reader.finish()?;
        printer.finish()?;
        Ok(f.into_allocations())
    }

    fn ty(&self) -> &FuncType {
        self.func
            .resources
            .sub_type_at(self.func.ty)
            .unwrap()
            .unwrap_func()
    }
}
