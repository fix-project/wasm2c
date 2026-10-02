use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    process::{Command, Output},
};

fn select(choices: &[&str]) -> PathBuf {
    for bin in choices {
        if let Ok(path) = which::which(bin) {
            return path;
        }
    }

    panic!("failed to find llvm-config")
}

fn command<P: AsRef<OsStr>>(cmd: P, args: &[&str]) -> Output {
    Command::new(cmd).args(args).output().unwrap()
}

fn output_to_path(output: Output) -> PathBuf {
    PathBuf::from(OsString::from_vec(output.stdout.trim_ascii().to_vec()))
}

fn output_to_trim(output: Output) -> String {
    String::from_utf8_lossy(output.stdout.trim_ascii()).to_string()
}

fn clang_configuration() -> (String, Vec<String>) {
    let resource_dir = output_to_trim(command("clang", &["-print-resource-dir"]));
    let probe = command("clang", &["-E", "-x", "c++", "-v", "/dev/null"]);
    let diagnostics = String::from_utf8_lossy(&probe.stderr);

    let includes = diagnostics
        .lines()
        .map(|l| l.trim())
        .skip_while(|line| *line != "#include <...> search starts here:")
        .take_while(|line| *line != "End of search list.")
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    (resource_dir, includes)
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn main() {
    println!("cargo::rerun-if-changed=src/jit.cpp");

    let llvm_config_bin = select(&["llvm-config-22", "llvm-config-23", "llvm-config"]);

    let llvm_dir = output_to_path(command(&llvm_config_bin, &["--prefix"]));
    let (resource_dir, include_paths) = clang_configuration();
    let include_initializer = format!(
        "{{ {} }}",
        include_paths
            .iter()
            .map(|path| quote(path))
            .collect::<Vec<_>>()
            .join(", ")
    );

    cc::Build::new()
        .cpp(true)
        .std("c++23")
        .compiler("clang++")
        .include(llvm_dir.join("include"))
        .define(
            "JIT_CLANG_RESOURCE_DIR",
            Some(quote(&resource_dir).as_str()),
        )
        .define(
            "JIT_SYSTEM_INCLUDE_PATHS",
            Some(include_initializer.as_str()),
        )
        .flag("-Wno-unused-parameter")
        .file("src/jit.cpp")
        .compile("jit");

    let library = llvm_dir.join("lib");
    println!("cargo::rustc-link-search=native={}", library.display());
    println!("cargo::rustc-link-lib=dylib=clang-cpp");
    println!("cargo::rustc-link-lib=dylib=LLVM");
    println!("cargo::rustc-link-arg=-Wl,-rpath,{}", library.display());
}
