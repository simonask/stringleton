use hashbrown::HashMap;

use stringleton::{Symbol, sym};
stringleton::enable!();

#[test]
#[cfg(feature = "alloc")]
fn basic() {
    let a = sym!(a);
    let b = sym!(b);
    let c = sym!(c);
    let a2 = sym!(a);

    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
    assert_eq!(a, a2);
}

#[test]
fn sym_macro() {
    let ident: Symbol = sym!(hello);
    let string: Symbol = sym!("hello");
    let dynamic = Symbol::new_static(&"hello");
    assert_eq!(ident, string);
    assert_eq!(ident, dynamic);

    let mut map = HashMap::new();
    map.insert(ident, 1);
    map.insert(string, 2);
    map.insert(dynamic, 3);
    assert_eq!(map.len(), 1);
    assert!(map.into_iter().eq([(ident, 3)]));

    assert_eq!(ident.to_string(), "hello");
    assert_eq!(ident.as_str(), "hello");

    let t = sym!(SYM_CACHE);
    assert_eq!(t, "SYM_CACHE");
}
