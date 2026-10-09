use anyhow::{Result, bail};
use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    fn run_program(
        source: *const c_char,
        source_length: usize,
        header: *const c_char,
        header_length: usize,
        passed: *mut bool,
    ) -> c_int;
}

// Compiles and runs C++ that sets one entry of `passed` per assertion with `run(bool* passed)`
pub fn run(source: &str, header: &str, passed: &mut [bool]) -> Result<()> {
    let c_source = CString::new(source)?;
    let c_header = CString::new(header)?;

    let err = unsafe {
        run_program(
            c_source.as_ptr(),
            source.len(),
            c_header.as_ptr(),
            header.len(),
            passed.as_mut_ptr(),
        )
    };

    if err != 0 {
        bail!("failed to run C++ source code");
    }

    Ok(())
}
