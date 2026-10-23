use anyhow::{Result, bail};
use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    fn run_program(
        source: *const c_char,
        source_length: usize,
        header: *const c_char,
        header_length: usize,
        passed: *mut u32,
    ) -> c_int;
}

// Compiles and runs C++ that returns a uint32_t of the number of successful commands
pub fn run(source: &[u8], header: &[u8]) -> Result<u32> {
    let c_source = CString::new(source)?;
    let c_header = CString::new(header)?;

    let mut num_successes: u32 = 0;

    let err = unsafe {
        run_program(
            c_source.as_ptr(),
            source.len(),
            c_header.as_ptr(),
            header.len(),
            std::ptr::addr_of_mut!(num_successes),
        )
    };

    if err != 0 {
        bail!("failed to run C++ source code");
    }

    Ok(num_successes)
}
