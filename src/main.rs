#![no_std]
#![no_main]

core::arch::global_asm!(include_str!("start.s"));

#[no_mangle]
extern "C" fn kmain(hart_id: usize, _dtb: *const u8) -> ! {
    println!();
    println!("Annwn v{}", env!("CARGO_PKG_VERSION"));
    println!("booting on hart {}", hart_id);

    loop {}
}

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

mod io;
