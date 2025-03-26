use stringleton::{Symbol, sym};

stringleton::enable!();

#[inline(never)]
#[unsafe(no_mangle)]
extern "C" fn get_symbol() -> Symbol {
    sym!("Hello, World!")
}

fn main() {
    println!("The symbol is: {}", get_symbol());
}
