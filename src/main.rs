#![no_std]
#![no_main]

use fdt::{Fdt, StructItem};

core::arch::global_asm!(include_str!("start.s"));

#[no_mangle]
extern "C" fn kmain(hart_id: usize, fdt: *const u8) -> ! {
    println!();
    println!("Annwn v{}", env!("CARGO_PKG_VERSION"));
    println!("booting on hart {}", hart_id);

    let fdt = unsafe { Fdt::from_ptr(fdt).unwrap() };
    for resv in fdt.memory_reservations() {
        println!(
            "Memory Reservation: address = {:#x}, size = {:#x}",
            resv.address, resv.size
        );
    }
    for item in fdt.struct_items() {
        match item {
            StructItem::BeginNode { name } => println!("FDT_BEGIN_NODE: name = {:?}", name),
            StructItem::EndNode => println!("FDT_END_NODE"),
            StructItem::Prop { name, value } => {
                println!("FDT_PROP: name = {:?}, value = {:?}", name, value)
            }
        }
    }

    loop {}
}

#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

mod fdt;
mod io;

mod util {
    pub fn align_up(value: usize, align: usize) -> usize {
        (value + align - 1) & !(align - 1)
    }
}
