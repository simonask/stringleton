use stringleton::sym;

stringleton::enable!();

pub fn foo() -> stringleton::Symbol {
    sym!(foo)
}
