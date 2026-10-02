use std::{path::PathBuf, process::Command};

fn llvm_config(option: &str) -> String {
    let output = Command::new("llvm-config")
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

fn main() {
    println!("cargo::rerun-if-changed=src/jit.cpp");

    assert!(
        llvm_config("--version").starts_with("22."),
        "require LLVM 22"
    );

    let clang = PathBuf::from(llvm_config("--bindir")).join("clang++");
    let libdir = llvm_config("--libdir");
    let clang_path = format!("\"{}\"", clang.display());

    cc::Build::new()
        .cpp(true)
        .std("c++23")
        .compiler(&clang)
        .include(llvm_config("--includedir"))
        .define("CLANG_PATH", Some(clang_path.as_str()))
        .flag("-Wno-unused-parameter")
        .file("src/jit.cpp")
        .compile("jit");

    println!("cargo::rustc-link-search=native={libdir}");
    println!("cargo::rustc-link-lib=dylib=clang-cpp");
    println!("cargo::rustc-link-lib=dylib=LLVM");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{libdir}");
}
