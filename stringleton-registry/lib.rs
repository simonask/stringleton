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
