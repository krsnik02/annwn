use core::{arch::asm, fmt::Write};

const SBI_EID_BASE: u32 = 0x10;
const SBI_EID_DBCN: u32 = 0x4442434e;

const SBI_FID_BASE_PROBE_EXTENSION: u32 = 3;
const SBI_FID_DBCN_CONSOLE_WRITE: u32 = 0;

fn sbi_probe_extension(eid: u32) -> bool {
    let value: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") SBI_EID_BASE,
            in("a6") SBI_FID_BASE_PROBE_EXTENSION,
            inlateout("a0") eid => _,
            lateout("a1") value,
        );
    }
    value != 0
}

/// SAFETY: `sbi_probe_extension(SBI_EID_DBCN)` has returned true.
unsafe fn sbi_debug_console_write(buf: &[u8]) -> Option<usize> {
    let error: usize;
    let value: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") SBI_EID_DBCN,
            in("a6") SBI_FID_DBCN_CONSOLE_WRITE,
            inlateout("a0") buf.len() => error,
            inlateout("a1") buf.as_ptr() as usize => value,
            in("a2") 0,
        )
    }
    if error == 0 {
        Some(value)
    } else {
        None
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::io::_print(::core::format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($fmt:expr) => { $crate::print!(::core::concat!($fmt, "\n")) };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(::core::concat!($fmt, "\n"), $($arg)*)
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    stdout().write_fmt(args).unwrap()
}

pub struct Stdout {
    has_dbcn: bool,
}

pub fn stdout() -> Stdout {
    Stdout {
        has_dbcn: sbi_probe_extension(SBI_EID_DBCN),
    }
}

impl core::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !self.has_dbcn {
            return Ok(());
        }

        let mut buf = s.as_bytes();
        while !buf.is_empty() {
            // SAFETY: the DBCN extension is present
            let written = unsafe { sbi_debug_console_write(buf) }.ok_or(core::fmt::Error)?;
            buf = &buf[written..];
        }

        Ok(())
    }
}
