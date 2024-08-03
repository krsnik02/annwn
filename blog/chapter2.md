# Building an RISC-V OS in Rust
## Chapter 2: Serial Output

### Organization
The first thing we want to be able to do is print messages on the serial output. Luckily for us, the [supervisor binary interface](https://drive.google.com/file/d/1U2kwjqxXgDONXk_-ZDTYzvsV-F_8ylEH/view) provided by OpenSBI provides an easy way to do this.

To keep our code organized, let's create a new module for this. Create a new file `src/io.rs` and add this line to `src/main.rs`.
```rust
// src/main.rs
mod io;
```

### Supervisor Binary Interface

The supervisor binary interface (or SBI) defines how a program running in S-mode can communicate with it's execution environment (or SEE). In our case, this is our kernel communication with OpenSBI.

The way we do this is by executing the `ecall` instruction, which will raise a synchronous exception, passing control back to OpenSBI. The values stored in the registers `a0`-`a7` tell the SEE what operation we want it to do, and it will return control back to us at the instruction after the `ecall` once it has done so.

SBI defines a series of extensions, each of which provides a number of functions. We select a function by passing a 32-bit extension id (EID) in register `a7`, and a 32-bit function id (FID) in register `a6`. Other parameters to the specific function are passed in registers `a0` through `a5` like in the normal C ABI. After the `ecall`, the SEE will have placed a return value in `a1` and an error code in `a0`. A error code of 0 means the command returned successfully while a negative value indicates an unsuccessful call.

For now, we need the functions `sbi_probe_extension` (FID 3) from the base extension (EID 0x10) and `sbi_debug_console_write` (FID 0) from extension DBCN (EID 0x4442434E). Let's define some constants to hold these values.
```rust
// src/io.rs
const SBI_EID_BASE: u32 = 0x10;
const SBI_EID_DBCN: u32 = 0x4442434e;

const SBI_FID_BASE_PROBE_EXTENSION: u32 = 3;
const SBI_FID_DBCN_CONSOLE_WRITE: u32 = 0;
```

Functions from the base extension are always present so `sbi_probe_extension` is a safe function. It takes an extension id and returns a bool specifying if that extension exists or not. It always returns an error code of 0 so we can just return the bool directly.

```rust
// src/io.rs
use core::arch::asm;

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

// ... (rest of file)
```

Functions from other extensions might not be present, so you must check the extension with `sbi_probe_extension` first. For this reason we mark `sbi_debug_console_write` as an `unsafe fn`.

This is a non-blocking call that takes in a length (in `a0`) and a 2*XLEN bit physical address (the low XLEN bits in `a1` and the high XLEN bits in `a2`) and returns either error code 0 and the number of bytes written or a non-zero error code if the write failed. We use physical addresses with less than XLEN=64 bits so we'll always set `a2` to zero.

```rust
// src/io.rs

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
```

### Formatted Output
Now let's write a `println` style wrapper around this to make it easier to use. We can copy the definition of these macros from the standard library. (The standard library actually uses a nightly macro `format_args_nl` in its definition of `println` now. This is how it was defined in previous versions.)
```rust
// src/io.rs
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
pub fn _print(args: core::fmt::Arguments<'_>) {
    todo!()
}
```

To fill in the `_print` function we implement the trait [Write](https://doc.rust-lang.org/beta/core/fmt/trait.Write.html) so we can use it's provided `write_fmt` method. 
```rust
// in core::fmt
trait Write {
    fn write_str(&mut self, s: &str) -> Result;

    fn write_char(&mut self, c: char) -> Result {
        // provided method
    } 

    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        // provided method
    }
}
```

We define a struct `Stdout` which can only be created outside of this module by calling the function `stdout` which checks if the DBCN extension is present. 
```rust
// src/io.rs

pub struct Stdout {
    has_dbcn: bool,
}

pub fn stdout() -> Stdout {
    Stdout {
        has_dbcn: sbi_probe_extension(SBI_EID_DBCN),
    }
}
```

We can now safely implement Write for Stdout. We implement `write_str` by repeatedly calling `sbi_debug_console_write` until either an io error occurs or we've written all bytes of the string. Finally we fill in `_print` by calling `write_fmt` on our `Stdout` value.
```rust
// src/io.rs

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

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments<'_>) {
    stdout().write_fmt(args).unwrap()
}
```

### Making sure it works

Let's use our new `println` functionality to print out a boot message in `kmain`! If you haven't seen it before, the `env` macro gets the value of an environment variable at compile-time.
```rust
// src/main.rs

#[no_mangle]
extern "C" fn kmain(hart_id: usize, _dtb: *const u8) -> ! {
    println!();
    println!("Annwn v{}", env!("CARGO_PKG_VERSION"));
    println!("booting on hart {}", hart_id);

    loop {}
}
```

Now when we run our kernel we should see our boot message printed out. 
```
$ cargo run
... (OpenSBI output)

Annwn v0.1.0
booting on hart 0
```

Since we didn't tell it otherwise, QEMU is only simulating a single hart and we'll always get hart 0. If we instead add `-smp 4` to the QEMU command line we can see that which hart we're given to initialize our kernel is randomly chosen by OpenSBI.
```
$ cargo run -- -smp 4
... (OpenSBI output)

Annwn v0.1.0
booting on hart 3          
```