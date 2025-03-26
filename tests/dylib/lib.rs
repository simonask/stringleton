#![cfg(all(test, not(miri)))]

use stringleton::{Symbol, sym};

stringleton::enable!();

#[test]
fn static_symbols_from_linked_dylib() {
    let syms = dynamic_library::symbols_a_b();
    assert_eq!(syms, [sym!(a), sym!(b)]);
}

#[allow(improper_ctypes)] // This is fine because it's the same Rust compiler on both sides.
unsafe extern "C" {
    fn cdylib_symbols_a_b(syms: &mut [Symbol; 2]);
}

#[test]
fn static_symbols_from_linked_cdylib() {
    let mut syms = [sym!(dummy), sym!(dummy)];
    unsafe {
        cdylib_symbols_a_b(&mut syms);
    };
    assert_eq!(syms, [sym!(a), sym!(b)]);
}
