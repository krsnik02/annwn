#![no_std]
#![no_main]

core::arch::global_asm!(include_str!("start.s"));

#[no_mangle]
extern "C" fn kmain(_hart_id: usize, _fdt: *const u8) -> ! {
    loop {}
}

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
