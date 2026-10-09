**Required packages (on Debian/Ubuntu) include**: 

`llvm-22` `llvm-22-dev` `clang-22` `libclang-cpp22` `libclang-cpp22-dev`


## Testing

`cargo run -p test-runner` runs all `.wast` files in `test-runner/tests` and reports their outcomes.

Pass a file or directory with `cargo run -p test-runner -- path/to/*`. 
The full test suite will be run if specific path is not passed.

Run full test suite with `cargo run -p test-runner -- --update` to update the "passing.txt" before submitting pull requests.

## Test suite

The WebAssembly 1.0 tests come from WebAssembly/testsuite mirror repository pinned to the first commit after official release of WebAssembly 1.0 from December 5th, 2019.
