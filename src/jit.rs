use anyhow::{Result, bail};
use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    fn run_program(
        source: *const c_char,
        source_length: usize,
        header: *const c_char,
        header_length: usize,
        result: *mut c_int,
    ) -> c_int;
}

pub fn run(source: &str, header: &str) -> Result<i32> {
    let c_source = CString::new(source)?;
    let c_header = CString::new(header)?;
    let mut result = 0;

    let err = unsafe {
        run_program(
            c_source.as_ptr(),
            source.len(),
            c_header.as_ptr(),
            header.len(),
            &mut result as *mut c_int,
        )
    };

    if err != 0 {
        bail!("failed to run C++ source code");
    }

    Ok(result)
}
