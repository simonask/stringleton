use foreign_crate::bar;
use foreign_crate_registry::foo;
use stringleton::sym;

// Forwarding to `foreign_crate_registry` through `foreign_crate`.
stringleton::enable!(foreign_crate);

#[test]
fn external_symbols_test() {
    assert_eq!(foo(), sym!(foo));
    assert_eq!(bar(), sym!(bar));
}
