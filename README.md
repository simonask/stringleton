# Stringleton String Interner

Extremely efficient string interning solution for Rust crates.

## Distinguishing characteristics

- Ultra fast: Getting the string representation of a `Symbol` is a lock-free
  memory load. No reference counting or atomics involved.
- Symbol literals (`sym!(...)`) are "free" at the call-site. Multiple
  invocations with the same string value are eagerly reconciled on program
  startup, using link-time tricks.
- Debugger friendly: If your debugger is able to display a plain Rust `&str`, it
  is capable of displaying `Symbol`.
- Dynamic library support: Symbols can be passed across dynamic linking
  boundaries (terms and conditions apply - see the documentation of
  `stringleton-dylib`).
- `no_std` support: `std` synchronization primitives used in the symbol registry
  can be replaced with `once_cell` and `spin`. (`alloc` is still needed by the
  internal hash table used in the registry.)
- `serde` support - symbols are serialized/deserialized as strings.
- Fast bulk-insertion of symbols at runtime.

## Good use cases

- You have lots of little strings that you need to frequently copy and compare.
- Your strings come from trusted sources.
- You need good debugger support for your symbols.

## Bad use cases

- You have an unbounded number of distinct strings, or strings coming from
  untrusted sources. Since symbols are never garbage-collected, this is a source
  of memory leaks, which is a denial-of-service hazard.
- You need a bit-stable representation of symbols that does not change between
  runs.

## Usage

Add `stringleton` as a dependency of your project.

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

## Efficiency

Stringleton tries to be as efficient as possible, but it may make different
tradeoffs than other string interning libraries.

In particular, Stringleton is optimized towards making the use of the
`sym!(...)` macro practically free.

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

## Name

The name is a portmanteau of "string" and "singleton".
