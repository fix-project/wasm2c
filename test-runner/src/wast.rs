use crate::jit;
use anyhow::{Result, bail};
use buffer_redux::{BufReader, BufWriter};
use clio::ClioPath;
use convert_case::ccase;
use std::io::Write;
use std::panic;
use wasm2c::{codegen, config::*};
use wast::core::{WastArgCore, WastRetCore};
use wast::parser::{self, ParseBuffer};
use wast::{Wast, WastArg, WastDirective, WastExecute, WastRet};

static W2CC: &'static str = "w2cc_";

#[test]
fn seven() -> Result<()> {
    let outcomes = run_test(include_str!("../samples/seven.wast"))?;
    anyhow::ensure!(
        outcomes.len() == 2 && outcomes.iter().all(|&(_, passed)| passed),
        "seven.wast failed"
    );
    Ok(())
}

// Line number and column number
pub type Location = (usize, usize);

// Runs every directive in a .wast file
pub fn run_test(text: &str) -> Result<Vec<(Location, bool)>> {
    let buffer = ParseBuffer::new(text)?;
    let wast = parser::parse::<Wast>(&buffer)?;
    let mut outcomes = Vec::new();
    let mut modules: Vec<Module> = Vec::new();

    for directive in wast.directives {
        let (line, column) = directive.span().linecol_in(text);
        let location = (line + 1, column + 1);
        match directive {
            WastDirective::Module(mut module) => modules.push(Module {
                location,
                wasm: module.encode().map_err(Into::into),
                asserts: Vec::new(),
                blocked: false,
            }),
            WastDirective::AssertReturn { exec, results, .. } => {
                // Check previous module
                match (modules.last_mut(), parse_assert_return(exec, results)) {
                    (Some(module), Ok(command)) if !module.blocked => {
                        module.asserts.push((location, command));
                    }
                    (Some(module), _) => {
                        module.blocked = true;
                        outcomes.push((location, false));
                    }
                    (None, _) => outcomes.push((location, false)),
                }
            }
            _ => {
                if let Some(module) = modules.last_mut() {
                    module.blocked = true;
                }
                outcomes.push((location, false));
            }
        }
    }

    for module in modules {
        outcomes.extend(run_module(module));
    }
    outcomes.sort_by_key(|&(location, _)| location);
    Ok(outcomes)
}

struct Module {
    location: Location,
    wasm: Result<Vec<u8>>,
    asserts: Vec<(Location, Command)>,
    blocked: bool,
}

fn run_module(module: Module) -> Vec<(Location, bool)> {
    let passed = module.wasm.ok().and_then(|wasm| {
        panic::catch_unwind(|| run_jit(&wasm, &module.asserts))
            .ok()?
            .ok()
    });

    let mut outcomes = vec![(module.location, passed.is_some())];
    let passed = passed.unwrap_or_else(|| vec![false; module.asserts.len()]);
    for ((location, _), passed) in module.asserts.iter().zip(passed) {
        outcomes.push((*location, passed));
    }
    outcomes
}

fn run_jit(wasm: &[u8], asserts: &[(Location, Command)]) -> Result<Vec<bool>> {
    let mut header = Vec::new();
    let mut source = Vec::new();

    {
        let mut output = Output {
            header: BufWriter::new(&mut header),
            source: BufWriter::new(&mut source),
            name: String::from("input"),
        };
        run_wasm2cc(wasm, &mut output)?;
        print_test(asserts, &mut output)?;
        output.header.flush()?;
        output.source.flush()?;
    }

    let mut passed = vec![false; asserts.len()];
    jit::run(
        &String::from_utf8_lossy(&source),
        &String::from_utf8_lossy(&header),
        &mut passed,
    )?;
    Ok(passed)
}

/* CODEGEN */
fn print_test(
    asserts: &[(Location, Command)],
    out: &mut Output<impl Write, impl Write>,
) -> Result<()> {
    print_includes(out)?;
    print_asserts(out)?;
    print_run(asserts, out)?;
    Ok(())
}

// called with one bool per assertion
fn print_run(
    asserts: &[(Location, Command)],
    out: &mut Output<impl Write, impl Write>,
) -> Result<()> {
    writeln!(out.source, "extern \"C\" void run(bool* passed) {{")?;

    let module = out.name.clone();
    print_construct_module(&module, out)?;

    for (i, (_, cmd)) in asserts.iter().enumerate() {
        match cmd {
            Command::AssertReturn {
                func,
                args,
                expected,
            } => {
                print_assert_return(i, func, args, expected, &module, out)?;
            }
        }
    }
    writeln!(out.source, "}}")?;

    Ok(())
}

fn print_includes(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let includes = ["<type_traits>"];
    for inc in includes {
        writeln!(out.source, "#include {}", inc)?;
    }
    Ok(())
}

fn print_assert_return(
    index: usize,
    func: &String,
    args: &Vec<Value>,
    expect: &Vec<Value>,
    curr_module: &String,
    out: &mut Output<impl Write, impl Write>,
) -> Result<()> {
    writeln!(out.source, "{{")?;

    let exp_type = print_expect(expect, out)?;
    print_result(func, args, curr_module, expect.is_empty(), out)?;

    if expect.is_empty() {
        writeln!(out.source, "passed[{index}] = true;")?;
    } else {
        writeln!(out.source, "ASSERT_TYPE(result, {});", exp_type)?;
        writeln!(out.source, "passed[{index}] = result == expect;")?;
    }

    writeln!(out.source, "}}")?;
    Ok(())
}

fn print_expect(expect: &Vec<Value>, out: &mut Output<impl Write, impl Write>) -> Result<String> {
    let mut type_str = String::new();
    let mut val_str = String::new();

    let exp_str = match expect.len() {
        0 => "".to_string(),
        1 => {
            type_str = cc_type(&expect[0]);
            format!("{} expect{{{}}}", &type_str, cc_value(&expect[0]))
        }
        _ => {
            type_str += "std::tuple<";
            for (i, exp) in expect.iter().enumerate() {
                if i > 0 {
                    type_str += ", ";
                    val_str += ", ";
                }
                type_str += &cc_type(exp);
                val_str += &cc_value(exp);
            }
            type_str += ">";
            format!("{type_str} expect{{{val_str}}}")
        }
    };

    writeln!(out.source, "{};\n", exp_str)?;
    Ok(type_str)
}

fn print_result(
    func: &String,
    args: &Vec<Value>,
    curr_module: &String,
    is_void: bool,
    out: &mut Output<impl Write, impl Write>,
) -> Result<()> {
    let mut call_str = if is_void {
        String::new()
    } else {
        String::from("auto result = ")
    };

    call_str += &format!("{curr_module}.{W2CC}{func}(");

    for i in 0..args.len() {
        if i > 0 {
            call_str += ", ";
        }
        let arg_str = match args[i] {
            Value::I32(val) => val.to_string(),
            Value::I64(val) => val.to_string(),
        };
        call_str += &arg_str;
    }

    call_str += ")";
    if is_void {
        writeln!(
            out.source,
            "static_assert(std::is_void_v<decltype({call_str})>);"
        )?;
    }
    writeln!(out.source, "{};\n", call_str)?;

    Ok(())
}

fn print_construct_module(name: &String, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(out.source, "{} {};\n", ccase!(pascal, name), name)?;
    Ok(())
}

fn print_asserts(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(
        out.source,
        "#define ASSERT_TYPE(var, T) \\\
    \n    static_assert(std::is_same<decltype(var), T>::value, #var \" must be \" #T)"
    )?;

    writeln!(out.source)?;

    Ok(())
}

/* SECTION 2: PARSE WAST */
enum Command {
    AssertReturn {
        func: String,
        args: Vec<Value>,
        expected: Vec<Value>,
    },
}

enum Value {
    I32(i32),
    I64(i64),
    // F32, F64
}

fn run_wasm2cc(bytes: &[u8], out: &mut Output<impl Write, impl Write>) -> Result<()> {
    // Must match Output::name: codegen qualifies member definitions with the
    // output name but names the class after the module name, and the JIT
    // compiles the pair as input.cc / input.hh.
    let mut config = Config {
        name: out.name.clone(),
        dest_dir: ClioPath::default(),
        reader: BufReader::new(bytes),
    };

    codegen::print_includes(out)?;
    codegen::print_typedefs(out)?;
    codegen::print_program(&mut config, out)?;
    Ok(())
}

fn parse_assert_return(exec: WastExecute, results: Vec<WastRet>) -> Result<Command> {
    let (func, args) = match exec {
        WastExecute::Invoke(invoke) => {
            if invoke.module.is_some() {
                bail!("named module invocations are not supported yet");
            }
            let func = invoke.name.to_string();
            let args = args_to_values(invoke.args)?;
            (func, args)
        }
        _ => {
            bail!("unsupported assert_return action");
        }
    };

    let expected = results_to_values(results)?;

    Ok(Command::AssertReturn {
        func,
        args,
        expected,
    })
}

fn args_to_values(args: Vec<WastArg>) -> Result<Vec<Value>> {
    let mut values = Vec::new();

    for arg in args {
        let val = match arg {
            WastArg::Core(ty) => match ty {
                WastArgCore::I32(val) => Value::I32(val),
                WastArgCore::I64(val) => Value::I64(val),
                _ => {
                    bail!("unsupported argument");
                }
            },
            WastArg::Component(_) => {
                bail!("unsupported argument");
            }
            _ => {
                bail!("unsupported argument");
            }
        };
        values.push(val);
    }
    Ok(values)
}

fn results_to_values(results: Vec<WastRet>) -> Result<Vec<Value>> {
    let mut values = Vec::new();

    for res in results {
        let val = match res {
            WastRet::Core(ty) => match ty {
                WastRetCore::I32(val) => Value::I32(val),
                WastRetCore::I64(val) => Value::I64(val),
                _ => {
                    bail!("unsupported result");
                }
            },
            WastRet::Component(_) => {
                bail!("unsupported result");
            }
            _ => {
                bail!("unsupported result");
            }
        };
        values.push(val);
    }

    Ok(values)
}

/* SECTION 3: */

/* UTILITIES */
fn cc_type(val: &Value) -> String {
    match val {
        Value::I32(_) => "i32".to_string(),
        Value::I64(_) => "i64".to_string(),
    }
}

fn cc_value(val: &Value) -> String {
    match val {
        Value::I32(val) => val.to_string(),
        Value::I64(val) => val.to_string(),
    }
}
