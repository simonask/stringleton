/// This is a "plain" dynamic library (C ABI), so it has to explicitly link
/// against a dynamic version of `stringleton-registry`. Otherwise, the Rust
/// compiler will include a separate copy of the crate in the dylib, meaning
/// that symbols from this crate will not be valid in the host crate.
use stringleton_dylib::{Symbol, sym};

stringleton_dylib::enable!();

#[unsafe(no_mangle)]
pub extern "C" fn cdylib_symbols_a_b(syms: &mut [Symbol; 2]) {
    _ = sym!(c);
    *syms = [sym!(a), sym!(b)];
}
