//! Dynamic linking support for Stringleton.
//!
//! This crate always produces a dynamic library, and it should be used by any
//! crate that ends up being a `cdylib`. When this appears somewhere in the
//! dependency graph, it causes the Rust compiler to produce a dynamic version
//! of `stringleton-registry`, which means that both uses of `stringleton` and
//! `stringleton-dylib` use the same symbol registry, so `Symbol`s can be safely
//! passed across the dynamic linking boundary.
//!
//! The host crate can safely use `stringleton` as a dependency, **except** when
//! dynamic libraries using `stringleton-dylib` are loaded at runtime (i.e.,
//! Rust cannot know that `stringleton-registry` should be dynamically linked).
//! In that case, the host crate should specify this crate as its dependency
//! instead of `stringleton`.

// Note: This perma-fails in rust-analyzer, but it's fine.
#[path = "../stringleton/lib.rs"]
mod lib_;

pub use lib_::*;
