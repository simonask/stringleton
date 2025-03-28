//! Registry helper crate for `stringleton`
//!
//! You probably don't need to use this crate directly. Use the
//! [`stringleton`](../stringleton) crate or the
//! [`stringleton-dylib`](../stringleton-dylib) crate instead.
//!
//! This crate exists to support both static and dynamic linking when using
//! `stringleton`.
//!
//! All binaries in a process must use the same symbol registry, so when a
//! dynamic library (`dylib`) dependency is using `stringleton`, this crate must
//! also be compiled as a dynamic library, which is ensured by the [linkage
//! rules](https://doc.rust-lang.org/reference/linkage.html).
//!
//! This only works automatically when such a dependency is "implicitly" linked
//! (i.e. it is a direct dependency in `Cargo.toml`). If dynamic libraries are
//! being loaded at runtime by the host process, Cargo must be instructed to
//! dynamically link against the registry, which can be achieved by using the
//! `stringleton-dylib` crate in the main binary instead of `stringleton`.
//!
//! Note that if a dependency is a `cdylib` (crate-type), that dependency must
//! explicitly link against `stringleton-dylib` for this trick to work. This is
//! not necessary when building a normal Rust `dylib`.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

mod registry;
mod site;
mod static_symbol;
mod symbol;

pub use registry::*;
pub use site::*;
pub use static_symbol::*;
pub use symbol::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    #[cfg(feature = "alloc")]
    fn new() {
        let a = Symbol::new("a");
        let b = Symbol::new("b");
        let a2 = Symbol::new("a");
        assert_eq!(a, a2);
        assert_ne!(a, b);
    }

    #[test]
    fn new_static() {
        static UNIQUE_SYMBOL: &str =
            "This is a globally unique string that exists nowhere else in the test binary.";

        let a = Symbol::new_static(&"a");
        let b = Symbol::new_static(&"b");
        let a2 = Symbol::new_static(&"a");
        assert_eq!(a, a2);
        assert_ne!(a, b);

        let unique = Symbol::new_static(&UNIQUE_SYMBOL);
        assert_eq!(
            std::ptr::from_ref(unique.inner()),
            std::ptr::from_ref(&UNIQUE_SYMBOL)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn address() {
        let a = Symbol::new_static(&"a");
        let a2 = Symbol::new(alloc::string::String::from("a"));
        assert_eq!(a, a2);
        assert_eq!(a.to_ffi(), a2.to_ffi());
        let a3 = Symbol::try_from_ffi(a.to_ffi()).unwrap();
        assert_eq!(a3, a);
    }
}
