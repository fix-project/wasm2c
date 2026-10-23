use crate::{OutputSpec, wasm2c};
use anyhow::{Result, bail};
use indoc::writedoc;
use itertools::Itertools;
use std::fmt::Display;
use std::io::Write;
use wast::core::{WastArgCore, WastRetCore};
use wast::lexer::Lexer;
use wast::parser::{self, ParseBuffer};
use wast::{Wast, WastArg, WastDirective, WastExecute, WastInvoke, WastRet};

// Convert wast script to C. Returns number of directives/commands.
pub fn wast2c<U: Write>(
    text: &str,
    output: &mut OutputSpec<U>,
    allow_confusing_unicode: bool,
) -> Result<usize> {
    let OutputSpec::Split { hh_include, hh, cc } = output else {
        bail!("cannot convert wast script to header-only output");
    };

    let mut lexer = Lexer::new(text);
    lexer.allow_confusing_unicode(allow_confusing_unicode);
    let buffer = ParseBuffer::new_with_lexer(lexer)?;
    let wast = parser::parse::<Wast>(&buffer)?;
    let num_directives = wast.directives.len();
    let mut module_output = OutputSpec::Unified(hh);
    let mut last_module_instance_name = None;

    // write preamble
    writedoc!(
        cc,
        "#include \"{hh_include}\"
         #include <print>

         unsigned int g_success_count;

         void assert_eq(auto x, auto y, int line_no) {{
           if (x == y) {{
             g_success_count++;
           }} else {{
             std::println(stderr, \"error, failed assertion on {{}}\", line_no);
           }}
         }}

         extern \"C\" unsigned int run(void) {{
         g_success_count = 0;\n"
    )?;

    // loop through every directive/command in the Wast script.
    // right now we only support "module" and "assert_return" comamnds
    for directive in wast.directives {
        let line_no = directive.span().linecol_in(text).0;
        match directive {
            WastDirective::Module(mut module) => {
                let name = format!("line{}", line_no);
                let instance_name = format!("instance_{name}");
                let class_name = wasm2c(&name, &module.encode()?, &mut module_output)?;
                writeln!(cc, "{class_name} {instance_name}{{}};")?; // TODO: provide imports
                writeln!(cc, "g_success_count++;")?;
                last_module_instance_name = Some(instance_name);
            }
            WastDirective::AssertReturn {
                exec:
                    WastExecute::Invoke(WastInvoke {
                        span: _,
                        module: None,
                        name,
                        args,
                    }),
                results,
                span,
            } => {
                let line_no = span.linecol_in(text).0;

                let inst = last_module_instance_name.as_ref().expect("previous module");
                writeln!(cc, "{{")?;

                match results.iter().as_slice() {
                    [] => {} // do nothing
                    [val] => writeln!(cc, "auto expected = {};", ret_value(val))?,
                    vals => writeln!(
                        cc,
                        "auto expected = std::tuple{{{}}};",
                        vals.iter().map(ret_value).format(", ")
                    )?,
                };

                writeln!(
                    cc,
                    "auto actual = {inst}.{name}({});",
                    args.iter().map(arg_value).format(", ")
                )?;

                if results.is_empty() {
                    writedoc!(
                        cc,
                        "static_assert(std::is_same_v<void, decltype(actual)>);
                         g_success_count++;\n"
                    )?;
                } else {
                    writedoc!(
                        cc,
                        "static_assert(std::is_same_v<decltype(expected), decltype(actual)>);
                         assert_eq(expected, actual, {line_no});\n"
                    )?;
                }

                writeln!(cc, "}}")?;
            }
            _ => {} // XXX ignore for now
        }
    }

    writeln!(cc, "return g_success_count;")?;
    writeln!(cc, "}}")?;

    Ok(num_directives)
}

fn arg_value(arg: &WastArg<'_>) -> impl Display {
    core::fmt::from_fn(move |f| match arg {
        WastArg::Core(WastArgCore::I32(v)) => write!(f, "{v}"),
        WastArg::Core(WastArgCore::I64(v)) => write!(f, "{v}"),
        _ => unimplemented!("wast arg"),
    })
}

fn ret_value(ret: &WastRet<'_>) -> impl Display {
    core::fmt::from_fn(move |f| match ret {
        WastRet::Core(WastRetCore::I32(v)) => write!(f, "i32({v})"),
        WastRet::Core(WastRetCore::I64(v)) => write!(f, "i64({v})"),
        _ => unimplemented!("wast ret"),
    })
}
