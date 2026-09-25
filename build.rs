use std::{
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    process::{Command, Stdio},
};

fn main() {
    println!("cargo::rerun-if-changed=src/jit.cpp");

    let prefix_output = Command::new("llvm-config")
        .arg("--prefix")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let llvm = PathBuf::from(OsString::from_vec(
        prefix_output.stdout.trim_ascii().to_vec(),
    ));

    println!("path: {llvm:?}");

    cc::Build::new()
        .cpp(true)
        .std("c++23")
        .compiler(llvm.join("bin/clang++"))
        .archiver(llvm.join("bin/llvm-ar"))
        .include(llvm.join("include"))
        .flag("-Wno-unused-parameter")
        .file("src/jit.cpp")
        .compile("jit");

    let library = llvm.join("lib");
    println!("cargo::rustc-link-search=native={}", library.display());
    println!("cargo::rustc-link-lib=dylib=clang-cpp");
    println!("cargo::rustc-link-lib=dylib=LLVM");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", library.display());
}
