// TODO:
// modules need to have their own names
// split the tests up by module, all asserts on that module called at once

use anyhow::{ Result };
use convert_case::ccase;
use std::io::{Write};
use std::path::Path;
use wast::core::{WastArgCore, WastRetCore};
use wast::parser::{ self, ParseBuffer };
use wast::{Wast, WastArg, WastDirective, WastExecute, WastRet};
use wasm2c::config::*;
use wasm2c::codegen::{compile};
use clio::ClioPath;
use buffer_redux::{ BufReader };

static W2CC: &'static str = "w2cc_";

fn main() -> Result<()> {
    let mut config = get_config()?;
    let mut output = get_output(&mut config)?;
    let dest = config.dest_dir.path();

    let text = read_wast()?;
    let commands = parse_wast(&text, dest)?;

    // what main.cc needs
    // class name
    // what to call it with (func name, params)
    // what to expect
    //

    print_test(commands, &mut output)?;

    Ok(())
}

/* CODEGEN */
fn print_test(commands: Vec<Command>, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    print_includes(out)?;
    print_module_includes(&commands, out)?;

    writeln!(out.source, "bool failed = false;")?;
    print_asserts(out)?;
    print_main(&commands, out)?;
    Ok(())
}

fn print_main(commands: &Vec<Command>, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(out.source, "int main() {{")?;

    let mut curr_module = &String::new();

    for cmd in commands {
        match cmd {
            Command::Module { name } => {
                print_construct_module(name, out)?;
                curr_module = name;
            },
            Command::AssertReturn { func, args, expected } => {
                print_assert_return(func, args, expected, curr_module, out)?;
            },
        }
    }
    print_status(out)?;
    writeln!(out.source, "}}")?;

    Ok(())
}

fn print_status(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(out.source, "if (!failed) std::cout << \"all tests passed\\n\";")?;
    writeln!(out.source, "return failed;")?;
    Ok(())
}

fn print_includes(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    let includes = ["<type_traits>", "<iostream>"];
    for inc in includes {
        writeln!(out.source, "#include {}", inc)?;
    }
    Ok(())
}

fn print_assert_return(
    func: &String, 
    args: &Vec<Value>, 
    expect: &Vec<Value>,
    curr_module: &String,
    out: &mut Output<impl Write, impl Write>
) -> Result<()> {
    let exp_type = print_expect(expect, out)?;
    print_result(func, args, curr_module, expect.is_empty(), out)?;
    
    if !expect.is_empty() {
        writeln!(out.source, "ASSERT_TYPE(result, {});", exp_type)?;
        writeln!(out.source, "ASSERT_VALUE(result, expect);")?;
    }

    Ok(())
}

fn print_expect(expect: &Vec<Value>, out: &mut Output<impl Write, impl Write>) -> Result<String> {
    let mut type_str = String::new();
    let mut val_str = String::new();

    let exp_str = match expect.len() {
        0 => { "".to_string() },
        1 => { 
            type_str = cc_type(&expect[0]);
            format!("{} expect{{{}}}", &type_str, cc_value(&expect[0]))
        },
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
    out: &mut Output<impl Write, impl Write>
) -> Result<()> {
    let mut call_str = 
        if is_void {
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
            Value::I32(val) => { val.to_string() },
            Value::I64(val) => { val.to_string() },
        };
        call_str += &arg_str;
    };

    call_str += ")";
    writeln!(out.source, "{};\n", call_str)?;

    Ok(())
}

fn print_construct_module(name: &String, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(out.source, "{} {};\n", ccase!(pascal, name), name)?;
    Ok(())
}

fn print_module_includes(commands: &Vec<Command>, out: &mut Output<impl Write, impl Write>) -> Result<()> {
    for cmd in commands {
        match cmd {
            Command::Module { name } => {
                writeln!(out.source, "#include \"{}.hh\"", name)?;
            },
            _ => {}
        }
    };
    writeln!(out.source)?;
    Ok(())
}

fn print_asserts(out: &mut Output<impl Write, impl Write>) -> Result<()> {
    writeln!(out.source, "#define ASSERT_TYPE(var, T) \\\
    \n    static_assert(std::is_same<decltype(var), T>::value, #var \" must be \" #T)")?;

    writeln!(out.source, "#define ASSERT_VALUE(result, expected)                \\\
    \n    do {{                                              \\\
    \n        if ((result) != (expected)) {{                 \\\
    \n            std::cerr << \"result != expected\\n\";       \\\
    \n            failed = true;                            \\\
    \n        }}                                            \\\
    \n    }} while (0)")?;

    writeln!(out.source)?;

    Ok(())
}

// TODO: custom input .wast file later but for now, this'll do
fn read_wast() -> Result<String> {
    let text = std::fs::read_to_string("test/assert.wast")?;
    Ok(text)
}

/* SECTION 2: PARSE WAST */
enum Command {
    Module {
        name: String,
    },

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

fn parse_wast(text: &str, dest: &Path) -> Result<Vec<Command>> {
    let buf = ParseBuffer::new(text)?;
    let wast = parser::parse::<Wast>(&buf)?;

    let mut commands = Vec::new();
    // let mut curr_module: &Command;

    let module_name = "module"; // TODO: this needs to change later

    for dir in wast.directives {
        let command = match dir {
            WastDirective::Module(mut module) => {
                let bytes = module.encode()?;
                let module = run_wasm2cc(&bytes, dest, module_name)?;
                // curr_module = &module;
                module
            },
            WastDirective::AssertReturn { exec, results, .. } => {
                parse_assert_return(exec, results)?
            },
            _ => { todo!() }
        };
        commands.push(command);
    }
    Ok(commands)
}

/* bytes of the module -> paths of .cc and .hh */
fn run_wasm2cc(bytes: &[u8], dest: &Path, name: &str) -> Result<Command> {
    // let tmp_dir = tempdir()?;
    let mut config = Config {
        name: name.to_string(), // TODO: change later idk
        dest_dir: ClioPath::new(dest)?,
        reader: BufReader::new(bytes) 
    };
    compile(&mut config)?;

    Ok(Command::Module { name: config.name })
}

fn parse_assert_return(exec: WastExecute, results: Vec<WastRet>) -> Result<Command> {
    
    let (func, args) = match exec {
        WastExecute::Invoke(invoke) => {
            let func = invoke.name.to_string();
            let args = args_to_values(invoke.args);
            (func, args)
        },
        _ => { todo!() },
    };

    let expected = results_to_values(results);

    Ok(Command::AssertReturn { func, args, expected })
}

fn args_to_values(args: Vec<WastArg>) -> Vec<Value> {
    let mut values = Vec::new();

    for arg in args {
        let val = match arg {
            WastArg::Core(ty) => {
                match ty {
                    WastArgCore::I32(val) => { Value::I32(val) },
                    WastArgCore::I64(val) => { Value::I64(val) },
                    _ => { unimplemented!() }
                }
            },
            WastArg::Component(_) => { unimplemented!() },
            _ => { unimplemented!() },
        };
        values.push(val);
    };
    values
}

fn results_to_values(results: Vec<WastRet>) -> Vec<Value> {
    let mut values = Vec::new();

    for res in results {
        let val = match res {
            WastRet::Core(ty) => {
                match ty {
                    WastRetCore::I32(val) => { Value::I32(val) },
                    WastRetCore::I64(val) => { Value::I64(val) },
                    _ => { unimplemented!()}
                }
            },
            WastRet::Component(_) => { unimplemented!() },
            _ => { unimplemented!() }
        };
        values.push(val);
    };

    values
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

