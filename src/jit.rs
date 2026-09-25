use anyhow::{Result, bail};
use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    fn run_program(source: *const c_char, length: usize, result: *mut c_int) -> c_int;
}

pub fn run(source: &str) -> Result<i32> {
    let c_source = CString::new(source)?;
    let mut result = 0;

    let err = unsafe { run_program(c_source.as_ptr(), source.len(), &mut result as *mut c_int) };

    if err != 0 {
        bail!("failed to run C++ source code");
    }

    Ok(result)
}
