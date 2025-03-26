//! Registry helper crate for `stringleton`
//!
//! This crate exists to support both static and dynamic linking when using
//! `stringleton`.
//!
//! All binaries must use the same registry, so when a dynamic library (`dylib`)
//! dependency is using `stringleton`, this crate must also be compiled as a
//! dynamic library, which is ensured by the [linkage
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

#[cfg(feature = "alloc")]
extern crate alloc;

mod registry;
mod site;
mod symbol;

pub use registry::*;
pub use site::*;
pub use symbol::*;
