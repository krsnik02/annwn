# Building a RISC-V OS in Rust
## Chapter 3: Parsing the Device Tree
### Device Tree Blob

Our kernel needs to know what sort of machine we're running on - it gets this information by parsing the [device tree]() blob. This describes, among others, the number and type of CPUs, the amount and location of any available RAM, and the locations of any memory-mapped devices. The location of this device tree blob is placed in `a1` by OpenSBI before entering our kernel, so it is the second argument to `kmain`.

Let's create a new file `src/dtb.rs` to hold our code which parses the device tree.

### Parsing the Header