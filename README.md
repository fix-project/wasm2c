**Required packages (on Debian/Ubuntu) include**: 

`llvm-22` `llvm-22-dev` `clang-22` `libclang-cpp22` `libclang-cpp22-dev`


## Testing

`cargo run -p test-runner` runs all `.wast` files in `test-runner/samples` and reports their outcomes.

Pass a file or directory with `cargo run -p test-runner -- path/to/*`
