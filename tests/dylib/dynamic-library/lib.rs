// This is a Rust dynamic library, so it can link to the normal `stringleton`
// crate, because the Rust compiler can automatically figure out that
// `stringleton-registry` should be dynamically linked.
use stringleton::{Symbol, enable, sym};

enable!();

pub fn symbols_a_b() -> [Symbol; 2] {
    [sym!(a), sym!(b)]
}
