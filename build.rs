fn main() {
    println!("cargo::rerun-if-changed=src/start.s");
    println!("cargo::rerun-if-changed=link.x");
}
