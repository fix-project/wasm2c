// Partly written by Claude Opus 5.5.

use itertools::Itertools;
use serde_json::json;
use std::os::unix::ffi::OsStrExt;
use std::{
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn llvm_config(option: &str) -> String {
    let output = Command::new("llvm-config-22")
        .arg(option)
        .output()
        .expect("install LLVM 22 and Clang 22 development packages");

    assert!(
        output.status.success(),
        "llvm-config {option}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("llvm-config output must be UTF-8")
        .trim()
        .to_owned()
}

const SOURCE: &str = "src/jit.cc";

const CC_WARNINGS: &[&str] = &[
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Weffc++",
    "-Wshadow",
    "-Wold-style-cast",
    "-Wconversion",
    "-Wsign-conversion",
    "-Wnull-dereference",
    "-Woverloaded-virtual",
    "-Wnon-virtual-dtor",
    "-Wcast-align",
    "-Wformat=2",
    "-Wimplicit-fallthrough",
    "-Wunused",
    "-Werror",
];

fn write_compile_commands(build: &cc::Build) {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    let tool = build.get_compiler();
    let file = manifest_dir.join(SOURCE);
    let arguments: Vec<String> = std::iter::once(tool.path().as_os_str())
        .chain(tool.args().iter().map(|a| a.as_os_str()))
        .map(|a| a.to_string_lossy().into_owned())
        .chain(["-c".to_owned(), file.display().to_string()])
        .collect();
    let entry = json!({
        "directory": manifest_dir.display().to_string(),
        "file": file.display().to_string(),
        "arguments": arguments,
    });
    let contents =
        serde_json::to_string_pretty(&[&entry]).expect("compile commands serialize to JSON") + "\n";
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"))
        .ancestors()
        .find(|dir| dir.join("CACHEDIR.TAG").is_file())
        .map(Path::to_path_buf)
        .expect("target dir")
        .join("compile_commands.json");
    // Only rewrite when something changed, so clangd doesn't reload needlessly.
    if fs::read_to_string(&path).ok().as_deref() != Some(contents.as_str()) {
        fs::write(&path, contents).expect("write compile_commands.json");
    }
}

fn c_string_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for &b in bytes {
        match b {
            b'"' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            0x20..=0x7e => out.push(b as char),
            _ => write!(out, "\\{b:03o}").unwrap(),
        }
    }
    out.push('"');
    out
}

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");

    assert!(
        llvm_config("--version").starts_with("22."),
        "require LLVM 22"
    );

    assert_eq!(
        llvm_config("--has-rtti"),
        "YES",
        "require LLVM built with RTTI (LLVM_ENABLE_RTTI=ON)"
    );

    let clang = PathBuf::from(llvm_config("--bindir")).join("clang++");
    let libdir = llvm_config("--libdir");
    let clang_path = c_string_literal(clang.as_os_str().as_bytes());

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++23")
        .compiler(&clang)
        .flag("-isystem") // don't warn about system headers
        .flag(llvm_config("--includedir"))
        .define("CLANG_PATH", Some(clang_path.as_str()))
        .define(
            "CC_WARNINGS",
            Some(
                CC_WARNINGS
                    .iter()
                    .map(|x| format!("\"{x}\""))
                    .join(",")
                    .as_str(),
            ),
        )
        .file(SOURCE);
    for flag in CC_WARNINGS {
        build.flag(flag);
    }

    write_compile_commands(&build);
    build.compile("jit");

    println!("cargo::rustc-link-search=native={libdir}");
    println!("cargo::rustc-link-lib=dylib=clang-cpp");
    println!("cargo::rustc-link-lib=dylib=LLVM");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{libdir}");
}
