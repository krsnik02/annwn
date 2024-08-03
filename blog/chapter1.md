# Building an RISC-V OS in Rust
## Chapter 1: Bare metal Rust

### Prequesites

In order to run our kernel we will need the QEMU emulator, which can be dowloaded from their website, [https://www.qemu.org/](https://www.qemu.org/). We will also be using a version of GDB which can work with RISC-V code, you can either use `gdb-multiarch` or get it as part of [riscv-gnu-toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain).

### The bare minimum


The first thing to do is create a new project. I've decided to call my kernel Annwn, after the mythological Welsh otherworld, but you can call it anything you wish.
```
$ cargo new annwn
    Creating binary (application) `annwn` package
note: see more `Cargo.toml` keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
$ cd annwn
```

Next we create the file `.cargo/config.toml` with the following contents.
```toml
[build]
target = "riscv64imac-unknown-none-elf"

[target.riscv64imac-unknown-none-elf]
runner = """
qemu-system-riscv64 
    -serial stdio
    -machine virt 
    -kernel
"""
```
This tells cargo to build for a bare-metal 64-bit RISC-V system, and that `cargo run` should run our program using QEMU's `virt` machine. Our kernel will run in supervisor mode; a program called [OpenSBI](https://github.com/riscv-software-src/opensbi) will setup machine mode for us before our code runs.

If we try to run our program now, however, we'll get some errors!
```
$ cargo run
   Compiling annwn v0.1.0 (/home/krsnik/riscv/kernel/annwn)
error[E0463]: can't find crate for `std`
  |
  = note: the `riscv64imac-unknown-none-elf` target may not support the standard library
  = note: `std` is required by `annwn` because it does not declare `#![no_std]`

error: cannot find macro `println` in this scope
 --> src/main.rs:2:5
  |
2 |     println!("Hello, world!");
  |     ^^^^^^^

error: `#[panic_handler]` function required, but not found

error: requires `sized` lang_item

For more information about this error, try `rustc --explain E0463`.
error: could not compile `annwn` (bin "annwn") due to 4 previous errors
```

This is because the target `riscv64imac-unknown-none-elf` doesn't support the standard library, and we need to tell rust that our program will run on the bare metal. Replace `src/main.rs` with the following contents.
```rust
#![no_std]
#![no_main]

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

The two attributes at the top say we don't use the standard library, and that we don't have a `main` function. The latter is required because Rust expects some setup to be done before entering `main` which we aren't going to do. 

Finally, `#[panic_handler]` tells Rust to call this function when a panic occurs. It must never return, which is what its return type `!` means, so we just loop forever.

Now if we run our program QEMU should successfully start and run OpenSBI.
```
$ cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
     Running `qemu-system-riscv64 -machine virt -serial stdio -kernel target/riscv64imac-unknown-none-elf/debug/annwn`
VNC server running on 127.0.0.1:5900

OpenSBI v1.4
   ____                    _____ ____ _____
  / __ \                  / ____|  _ \_   _|
 | |  | |_ __   ___ _ __ | (___ | |_) || |
 | |  | | '_ \ / _ \ '_ \ \___ \|  _ < | |
 | |__| | |_) |  __/ | | |____) | |_) || |_
  \____/| .__/ \___|_| |_|_____/|____/_____|
        | |
        |_|

Platform Name             : riscv-virtio,qemu
Platform Features         : medeleg
... (more OpenSBI output)
```

### Linker scripts and assembly, oh my!

At this point we haven't written any code yet, so our kernel does absolutely nothing. In fact, we haven't even written any code for it to run!

When OpenSBI is done setting up machine mode for us, it enters supervisor mode and tries to jump to our code. Since we haven't told it where to expect the code, it seems to think it will be at memory address `0x10000`, so it executes whatever instruction happens to be there. 

We haven't put anything there, so it's likely that that instruction or one soon after will not be a valid instruction! When the cpu tries to execute an invalid instruction it generates a fault, which OpenSBI helpfully handles be resetting the machine and jumping back to our kernel, at which point the process repeats.

Let's fix this and actually give it some working code!

The first thing we must do is tell the linker where in memory to place our code and data. This is done by writing a file called a linker script. We create a file `link.x` with the following contents.
```ld
/* link.x */
MEMORY {
    RAM : ORIGIN = 0x80200000, LENGTH = 128M
    FLASH : ORIGIN = 0x20000000, LENGTH = 16M
}

SECTIONS {
    .text ORIGIN(FLASH) : {
        _stext = .;
        KEEP(*(.text.init))
        *(.text .text.*)
    } > FLASH

    .rodata : ALIGN(4K) {
        _srodata = .;
        *(.srodata .srodata.*)
        *(.rodata .rodata.*)
    } > FLASH

    .data : ALIGN(8) {
        _sidata = LOADADDR(.data);
        _sdata = .;
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.*)
        *(.data .data.*)
        . = ALIGN(8);
        _edata = .;
    } > RAM AT > FLASH

    .bss (NOLOAD) : ALIGN(8) {
        _sbss = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        _ebss = .;
    } > RAM

    .stack (NOLOAD) : ALIGN(8) {
        . = . + 4K;
        _sstack = .;
    } > RAM

    .heap (NOLOAD) : ALIGN(4K) {
        _sheap = .;
    } > RAM

    .eh_frame (INFO) : { KEEP(*(.eh_frame)) }
    .eh_frame_hdr (INFO) : { *(.eh_frame_hdr) }
}
```

Wow, that's a lot! Let's break this down.

This first part of this file defines some memory regions.
```ld
MEMORY {
    FLASH : ORIGIN = 0x20000000, LENGTH = 16M
    RAM : ORIGIN = 0x80200000, LENGTH = 16M
}
```
 The first region `FLASH` is 16 MiB of read-only memory starting at address `0x20000000`, and the second region `RAM` is a 16 MiB region of main memory starting at `0x80200000`. 

This is followed by `SECTIONS` block which tells the linker which section of our program to place in what memory region.

```ld
SECTIONS {
    .text ORIGIN(FLASH) : {
        _stext = .;
        KEEP(*(.text.init))
        *(.text .text.*)
    } > FLASH
```
We start with a section called `.text` which contains our actual program code and gets placed at the very beginning of the flash memory. We've made sure to place the code section `.text.init` as the first thing in this section, this will be where we place our kernel entry point for OpenSBI to jump to.

This block also defines a symbol `_stext` which can be referred to in our code, whose address will be the start of the section.


```ld
    .rodata : ALIGN(4K) {
        _srodata = .;
        *(.srodata .srodata.*)
        *(.rodata .rodata.*)
    } > FLASH
```
The next section, `.rodata`, contains the program's read-only data. This is where things like string literals and `const`s get stored. We put this in the flash immediately after the code, aligned to a 4 KiB boundary so that we can give it different permissions from `.text` when we implement paging.

Like before, we define a symbol `_srodata` at the start of this section.

```ld
    .data : ALIGN(8) {
        _sidata = LOADADDR(.data);
        _sdata = .;
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.* .sdata2 .sdata2.*)
        *(.data .data.*)
        . = ALIGN(8);
        _edata = .;
    } > RAM AT > FLASH
```
This is where things get interesting. The `.data` section is where all of the data which has an initial value but can be modified at run-time gets placed. Because we're running with Qemu we could have told it to place this directly in RAM, but that wouldn't work if we were to ever want to run on real hardware. 

Instead we tell the linker that when the program is running this data will be located in RAM, but the linker should _actually_ store the data in flash immediately after the `.rodata` section. Our program is responsible for moving the data there before using the data.

We define symbols `_sdata`, `_edata` and `_sidata` to help us do this. `_sdata` and `_edata` are at the beginning and end of the region in RAM where we need to move the data, and `_sidata` is at the start of the region in flash where the linker actually put the data.

We also define a symbol called `__global_pointer$`, located 2 KiB after the start of the data section in RAM. This pointer is used for something called _linker relaxation_, and we need to ensure that the `gp` register stores this location.

```ld
    .bss (NOLOAD) : ALIGN(8) {
        _sbss = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        _ebss = .;
    } > RAM
```
The `.bss` section is where all data with an initial value of zero lives. Since this data starts with a known value it doesn't need to be stored in the binary file or loaded into ram; our program is responsible for initializing it with zeros tho. Like earlier, we define symbols `_sbss` and `_ebss` at the start and end of this section to help us do so.

```ld
    .stack (NOLOAD) : ALIGN(8) {
        . = . + 4K;
        _sstack = .;
    } > RAM

    .heap (NOLOAD) : ALIGN(4K) {
        _sheap = .;
    } > RAM
}
```
Finally we define two non-standard sections, `.stack` and `.heap`. These sections are only here to help us define two symbols `_sstack` which is located at the top of our 4 KiB stack, and `_sheap` at the beginning of the free memory which we will use for our kernel's heap.


We need to tell cargo to use our linker script, so add the line
```toml
rustflags = ["-C", "link-arg=-Tlink.x"]
``` 
to the `[build]` section of `.config/cargo.toml`.


Now we need to write the first code we want our kernel to execute. This code is responsible for doing the necessary initialization I mentioned earlier before we can enter Rust code.
```assembly
# src/start.s
.section .text.init
.global _start

_start:
    # set gp
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    # set sp
    la sp, _sstack

    # move data to ram
    la t0, _sidata
    la t1, _sdata
    la t2, _edata
1:  beq t1, t2, 2f
    ld t3, 0(t0)
    sd t3, 0(t1)
    addi t0, t0, 8
    addi t1, t1, 8
    j 1b
2:

    # zero bss
    la t1, _sbss
    la t2, _ebss
3:  beq t1, t2, 4f
    sd zero, 0(t1)
    addi t1, t1, 8
    j 3b
4:

    j kmain
```

Let's break this down again!

The first two lines contain assembler directives. 
```assembly 
.section .text.init
.global _start
``` 
The first line says that the following code is part of the `.text.init` section we told the linker to place at the very beginning of flash, and the second line makes `_start` a global symbol which can be referred to by other programs.

```assembly
_start:
    # set gp
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    # set sp
    la sp, _sstack
```
This first line `_start:` is a label, whenever we refer to the symbol `_start` we mean the instructions right here. 

The instruction sets `gp` to the address `__global_pointer$`. We need to surround the actual instruction `la gp, __global_pointer$` with some assembler directives which temporially disable linker relaxation, otherwise the linker might decide to replace this instruction with a `gp`-relative access, `mv gp, gp`, which won't do what we want.

Next, we set `sp` to the top of our stack.

```assembly
    # move data to ram
    la t0, _sidata
    la t1, _sdata
    la t2, _edata
1:  beq t1, t2, 2f
    ld t3, 0(t0)
    sd t3, 0(t1)
    addi t0, t0, 8
    addi t1, t1, 8
    j 1b
2:
```
Now comes a loop which copies the `.data` section from read-only flash into ram. Register `t0` holds the current position in flash, `t1` the current position in ram, and `t2` the address just past the end of the region in ram we need to copy to.

This assembly is equivalent to the following C code:
```c
    for (uint64_t *src = _sidata, *dest = _sdata; dest != _edata; ++src, ++dest) {
        *dest = *src;
    }
```
Each time through the loop we copy a 64-bit (8 byte) value from the address in `t0` to the address in `t1`, and then increment both `t0` and `t1` by 8 bytes. We do this until `t1` and `t2` are equal, at which point we break out of the loop. 

The labels `1:` and `2:` are local labels, referred to as `1b` (noting that `1:` is prior to this point in the assembly) and `2f` (noting that `2:` is after this point in the assembly) respectively.

```assembly
    # zero bss
    la t1, _sbss
    la t2, _ebss
3:  beq t1, t2, 4f
    sd zero, 0(t1)
    addi t1, t1, 8
    j 3b
4:

    # jump to our Rust code
    j kmain
```
Now we have another loop which zeros out the `.bss` section. The only difference is that instead of loading a value from flash we store a hardcoded 0. The `zero` register in RISC-V is hardwired to always store a value of zero.

Finally, we jump to the symbol `kmain` which we will define in Rust code!

### Finally in Rust code

We add the following lines to `src/main.rs`.

```rust
// src/main.rs

core::arch::global_asm!(include_str!("start.s"));

#[no_mangle]
extern "C" fn kmain(_hart_id: usize, _dtb: *const u8) -> ! {
    loop {}
}
```

The first line includes our `src/start.s` file inside the `global_asm!` macro. `global_asm!` is part of Rust's excellent inline assembly system and doing this tells Rust about our assembly file so we don't have to invoke the assembler ourself.

The rest is the definition of the `kmain` function which our assembly code jumps to. The `#[no_mangle]` attribute tells us to use the name `kmain` directly as our symbol name, otherwise Rust would apply its [name-mangling](https://doc.rust-lang.org/stable/rustc/symbol-mangling/index.html)  scheme and we would have needed to say something like `_ZN5annwn5kmain_17h87d1c4b7efbac9d2E` in our assembly instead!

We also specify that `kmain` uses the C calling convention, defined in the [RISC-V ABI specification](https://drive.google.com/file/d/1Ja_Tpp_5Me583CGVD-BIZMlgGBnlKU4R/view), which passes arguments in registers `a0`, `a1`, `a2`, etc and never returns. You may note that we've defined two arguments, `hart_id` and `fdt`, but our assembly never set `a0` and `a1`; this is because OpenSBI set these two registers to the hart id of the current hart and the location of the [device tree](https://github.com/devicetree-org/devicetree-specification/releases)  binary blob respectively before handing control over to us.

If you run this code you'll get exactly the same output as before, but now instead of repeatedly trying to execute random memory as code we're actually in a controlled inifinite loop. You can check this by attaching a GDB debugger to our Qemu session.

Run `cargo run -- -S -s` to pass the extra arguments `-S -s` to the qemu command line. These arguments tell qemu to setup a GDB server on port `localhost:1234` and to not execute anything until the debugger tells it to. Now we can just `gdb` in another shell.
```
$ riscv64-unknown-elf-gdb target/riscv64imac-unknown-none-elf/debug/annwn
GNU gdb (GDB) 14.2
Copyright (C) 2023 Free Software Foundation, Inc.
License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.
Type "show copying" and "show warranty" for details.
This GDB was configured as "--host=x86_64-pc-linux-gnu --target=riscv64-unknown-elf".
Type "show configuration" for configuration details.
For bug reporting instructions, please see:
<https://www.gnu.org/software/gdb/bugs/>.
Find the GDB manual and other documentation resources online at:
    <http://www.gnu.org/software/gdb/documentation/>.

For help, type "help".
Type "apropos word" to search for commands related to "word"...
Reading symbols from target/riscv64imac-unknown-none-elf/debug/annwn...
(gdb) target remote :1234
Remote debugging using :1234
0x0000000000001000 in ?? ()
(gdb) b *kmain
Breakpoint 1 at 0x2000005a: file src/main.rs, line 7.
(gdb) display /i $pc
1: x/i $pc
=> 0x1000:      auipc   t0,0x0
(gdb) c
Continuing.

Breakpoint 1, annwn::kmain (_hart_id=0, _fdt=0x0) at src/main.rs:7
7       extern "C" fn kmain(_hart_id: usize, _fdt: *const u8) -> ! {
1: x/i $pc
=> 0x2000005a <annwn::kmain>:   addi    sp,sp,-16
```
We see that we've successfully entered our Rust code, and if we step forward a few instructions we end up in our infinite loop with an instruction which jumps to itself.
```
(gdb) si
0x000000002000005c      7       extern "C" fn kmain(_hart_id: usize, _fdt: *const u8) -> ! {
1: x/i $pc
=> 0x2000005c <annwn::kmain+2>: sd      a0,0(sp)
(gdb)
0x000000002000005e      7       extern "C" fn kmain(_hart_id: usize, _fdt: *const u8) -> ! {
1: x/i $pc
=> 0x2000005e <annwn::kmain+4>: sd      a1,8(sp)
(gdb) 
8           loop {}
1: x/i $pc
=> 0x20000060 <annwn::kmain+6>: j       0x20000062 <annwn::kmain+8>
(gdb) 
0x0000000020000062      8           loop {}
1: x/i $pc
=> 0x20000062 <annwn::kmain+8>: j       0x20000062 <annwn::kmain+8>
(gdb)
```

We've now sucessfully created a program which runs on a bare-metal RISC-V and enters Rust code! If you start playing around with it, you may notice that cargo doesn't rebuild our project if we change either of `link.x` or `src/start.s`. To make it do so, we add a build script `build.rs` containing the following code.
```rust
// build.rs
fn main() {
    println!("cargo::rerun-if-changed=src/start.s");
    println!("cargo::rerun-if-changed=link.x");
}
```

The code for this project can be found at [https://github.com/krsnik02/annwn](https://github.com/krsnik02/annwn).