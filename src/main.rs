#![no_std]
#![no_main]

use fdt::{Fdt, FdtNode};

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

    let root = fdt.root_node();
    show_node(root, 0);

    fn indent(depth: usize) {
        for _ in 0..depth {
            print!("    ");
        }
    }
    loop {}

    fn show_node(node: FdtNode<'_>, depth: usize) {
        indent(depth);
        println!("{} : {{", node.name);
        for prop in node.properties() {
            indent(depth);
            println!("    {} = {:?};", prop.name, prop.value);
        }
        for node in node.children() {
            show_node(node, depth + 1);
        }
        indent(depth);
        println!("}};");
    }
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
