use anyhow::{Result, bail};
use std::ffi::c_int;

unsafe extern "C" {
    fn init_with_precompiled_headers(includes: *const u8, includes_length: usize) -> c_int;

    fn run_program(
        header: *const u8,
        header_length: usize,
        source: *const u8,
        source_length: usize,
        passed: *mut u32,
    ) -> c_int;
}

pub fn init(includes: &[u8]) -> Result<()> {
    let err = unsafe { init_with_precompiled_headers(includes.as_ptr(), includes.len()) };
    if err != 0 {
        bail!("failed to precompile C++ headers");
    }
    Ok(())
}

// Compiles and runs C++ that returns a uint32_t of the number of successful Wast directives/commands
// (or error if the C++ doesn't compile or run).
pub fn run(header: &[u8], source: &[u8]) -> Result<u32> {
    let mut num_successes: u32 = 0;

    let err = unsafe {
        run_program(
            header.as_ptr(),
            header.len(),
            source.as_ptr(),
            source.len(),
            std::ptr::addr_of_mut!(num_successes),
        )
    };

    if err != 0 {
        bail!("failed to run C++ source code");
    }

    Ok(num_successes)
}
