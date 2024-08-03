MEMORY {
    FLASH : ORIGIN = 0x20000000, LENGTH = 16M
    RAM : ORIGIN = 0x80200000, LENGTH = 16M
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
        . = . + 8K;
        _sstack = .;
    } > RAM

    .heap (NOLOAD) : ALIGN(4K) {
        _sheap = .;
    } > RAM
}