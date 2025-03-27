# Stringleton

Extremely efficient string interning solution for Rust crates.

*String interning:* The technique of representing all strings which are equal by
a pointer or ID that is unique to the *contents* of that strings, such that O(n)
string equality check becomes a O(1) pointer equality check.

Interned strings in Stringleton are called "symbols", in the tradition of Ruby.

## Distinguishing characteristics

- Ultra fast: Getting the string representation of a `Symbol` is a lock-free
  memory load. No reference counting or atomics involved.
- Symbol literals (`sym!(...)`) are "free" at the call-site. Multiple
  invocations with the same string value are eagerly reconciled on program
  startup using linker tricks.
- Symbols are tiny. Just a single pointer - 8 bytes on 64-bit platforms.
- Symbols are trivially copyable - no reference counting.
- No size limit - symbol strings can be arbitrarily long (i.e., this is not a
  "small string optimization" implementation).
- Debugger friendly: If your debugger is able to display a plain Rust `&str`, it
  is capable of displaying `Symbol`.
- Dynamic library support: Symbols can be passed across dynamic linking
  boundaries (terms and conditions apply - see the documentation of
  `stringleton-dylib`).
- `no_std` support: `std` synchronization primitives used in the symbol registry
  can be replaced with `once_cell` and `spin`. *See below for caveats.*
- `serde` support - symbols are serialized/deserialized as strings.
- Fast bulk-insertion of symbols at runtime.

## Good use cases

- You have lots of little strings that you need to frequently copy and compare.
- Your strings come from trusted sources.
- You want good debugger support for your symbols.

## Bad use cases

- You have an unbounded number of distinct strings, or strings coming from
  untrusted sources. Since symbols are never garbage-collected, this is a source
  of memory leaks, which is a denial-of-service hazard.
- You need a bit-stable representation of symbols that does not change between
  runs.
- Consider if `smol_str` or `cowstr` is a better fit for such use cases.

## Usage

Add `stringleton` as a dependency of your project, and then you can do:

```rust,ignore
use stringleton::{sym, Symbol};

// Enable the `sym!()` macro in the current crate. This should go at the crate root.
stringleton::enable!();

let foo = sym!(foo);
let foo2 = sym!(foo);
let bar = sym!(bar);
let message = sym!("Hello, World!");
let message2 = sym!("Hello, World!");

assert_eq!(foo, foo2);
assert_eq!(bar.as_str(), "bar");
assert_eq!(message, message2);
assert_eq!(message.as_str().as_ptr(), message2.as_str().as_ptr());
```

## Crate features

- **std** *(enabled by default)*: Use synchronization primitives from the
  standard library. Implies `alloc`. When disabled, `critical-section` and
  `spin` must both be enabled *(see below for caveats)*.
- **alloc** *(enabled by default)*: Support creating symbols from `String`.
- **serde**: Implements `serde::Serialize` and `serde::Deserialize` for symbols,
  which will be serialized/deserialized as plain strings.
- **debug-assertions**: Enables expensive debugging checks at runtime - mostly
  useful to diagnose problems in complicated linker scenarios.
- **critical-section**: When `std` is not enabled, this enables `once_cell` as a
  dependency with the `critical-section` feature enabled. Only relevant in
  `no_std` environments. *[See `critical-section` for more
  details.](https://docs.rs/critical-section/latest/critical_section/)*
- **spin**: When `std` is not enabled, this enables `spin` as a dependency,
  which is used to obtain global read/write locks on the symbol registry. Only
  relevant in `no_std` environments (and is a pessimization in other
  environments).

## Efficiency

Stringleton tries to be as efficient as possible, but it may make different
tradeoffs than other string interning libraries. In particular, Stringleton is
optimized towards making the use of the `sym!(...)` macro practically free.

Consider this function:

```rust,ignore
fn get_symbol() -> Symbol {
    sym!("Hello, World!")
}
```

This compiles into a single load instruction. Using `cargo disasm` on x86-64
(Linux):

```asm
get_symbol:
  8bf0    mov  rax, qword ptr [rip + 0x52471]
  8bf7    ret
```

This is "as fast as it gets", but the price is that all symbols in the program
are deduplicated when the program starts. Any theoretically faster solution
would need fairly deep cooperation from the compiler aimed at this specific use
case.

Also, symbol literals are *always* a memory load. The compiler cannot perform
optimizations based on the contents of symbols, because it doesn't know how they
will be reconciled until link time. For example, while `sym!(a) != sym!(a)` is
always false, the compiler cannot eliminate code paths relying on that.

## Dynamic libraries

Stringleton relies on magical linker tricks (supported by `linkme` and `ctor`)
to minimize the cost of the `sym!(...)` macro at runtime. These tricks are
broadly compatible with dynamic libraries, but there are a few caveats:

1. When a Rust `dylib` crate appears in the dependency graph, and it has
   `stringleton` as a dependency, things should "just work", due to Rust's
   [linkage rules](https://doc.rust-lang.org/reference/linkage.html).
2. When a Rust `cdylib` crate appears in the dependency graph, Cargo seems to be
   a little less clever, and the `cdylib` dependency may need to use the
   `stringleton-dylib` crate instead. Due to Rust's linkage rules, this will
   cause the "host" crate to also link dynamically with Stringleton, and
   everything will continue to work.
3. When a library is loaded dynamically at runtime, and it does not appear in
   the dependency graph, the "host" crate must be prevented from linking
   statically to `stringleton`, because it would either cause duplicate symbol
   definitions, or worse, the host and client binaries would disagree about
   which `Registry` to use. To avoid this, the *host* binary can use
   `stringleton-dylib` explicitly instead of `stringleton`, which forces dynamic
   linkage of the symbol registry.
4. Dynamically *unloading* libraries is extremely risky (`dlclose()` and
   similar). Unloading a library that has any calls to the `sym!(..)` or
   `static_sym!(..)` macros is instant UB. Such a library can in principle use
   `Symbol::new()`, but probably not `Symbol::new_static()`.

To summarize:

1. When no dynamic libraries are present in the project, it is always best to
   use `stringleton` directly.
2. When only normal Rust dynamic libraries (`crate-type = ["dylib"]`) are
   present, it is also fine to use `stringleton` directly - Cargo and rustc will
   figure out how to link things correctly.
3. `cdylib` dependencies should use `stringleton-dylib`. The host can use
   `stringleton`.
4. When loading dynamic libraries at runtime, both sides should use
   `stringleton-dylib` instead of `stringleton`.
5. Do not unload dynamic libraries at runtime unless you are really, really sure
   what you are doing.

## `no_std` caveats

Stringleton works in `no_std` environments, but it does fundamentally require
two things:

1. Allocator support, in order to maintain the global symbol registry. This is a
   `hashbrown` hash map.
2. Some synchronization primitives to control access to the global symbol
   registry when new symbols are created.

The latter can be supported by the `spin` and `critical-section` features:

- `spin` replaces `std::sync::RwLock`, and is almost always a worse choice when
  `std` is available.
- `critical-section` replaces `std::sync::OnceLock` with
  [`once_cell::sync::OnceCell`](https://docs.rs/once_cell/latest/once_cell/sync/struct.OnceCell.html),
  and enables the `critical-secion` feature of `once_cell`. Using
  `critical-section` requires additional work, because you must manually link in
  a crate that provides the relevant synchronization primitive for the target
  platform.

Do not use these features unless you are familiar with the tradeoffs.

## Name

The name is a portmanteau of "string" and "singleton".
