**Required packages (on Debian/Ubuntu) include**: 

`build-essential` `llvm-22` `llvm-22-dev` `clang-22` `libclang-cpp22`
`libclang-22-dev` `libclang-cpp22-dev`

## Testing

`cargo run -p wast2c-jit` runs `.wast` files and reports their outcomes.

`git submodule update --init` will set up the testsuite submodule.

Pass a file or directory with `cargo run -p wast2c-jit -- path/to/*`. 

## Test suite

The WebAssembly 1.0 tests come from the WebAssembly/testsuite repository
pinned to the first commit after the release of WebAssembly 1.0 from
December 5th, 2019.
