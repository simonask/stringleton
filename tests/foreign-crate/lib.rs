stringleton::enable!(::foreign_crate_registry);

pub fn bar() -> stringleton::Symbol {
    stringleton::sym!(bar)
}
