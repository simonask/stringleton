use core::{hash::Hash, ptr::NonNull};

#[cfg(all(not(feature = "alloc"), feature = "std"))]
use std as alloc;

#[cfg(feature = "alloc")]
use alloc::{borrow::Cow, string::String};

use crate::Registry;

/// Interned string with very fast comparison and hashing.
///
/// Symbols should typically be used as extremely fast program-internal
/// identifiers.
///
/// # Comparison
///
/// Symbol comparison is just comparison between single pointers, which are
/// guaranteed to be identical for identical strings. No O(n) string comparisons
/// occur.
///
/// The implementation of `Ord` between symbols also does **not** perform string
/// comparison, but rather compares pointer values. However, the
/// `PartialOrd<str>` implementations do compare strings.
///
/// # Hashing
///
/// The hash value of a symbol is not predictable, as it depends on specific
/// pointer values, which are determined by both linking order and heap layout
/// (for dynamically created symbols). In particular, the hash value of `Symbol`
/// is **not** the same as the hash value of the underlying `str`.
///
/// For this reason, `Symbol` does not implement `Borrow<str>`, which would
/// imply that it would hash to the same value as its corresponding string
/// value. To prevent accidents, `Symbol` also does not implement `Deref<Target
/// = str>` (_this restriction may be lifted in future_).
///
/// The hash value of symbols may change even between invocations of the same
/// binary, so should not be relied upon in any way.
///
/// # Leaks
///
/// Once created, symbols are never freed, and there is no way to
/// "garbage-collect" symbols. This means that dynamically creating symbols from
/// user input or runtime data is a great way to create memory leaks. Use only
/// for static or semi-static identifiers, or otherwise trusted input.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Symbol(&'static &'static str);

impl Symbol {
    /// Create a deduplicated symbol at runtime.
    ///
    /// All calls to this function with the same string argument will return a
    /// bit-identical `Symbol`.
    ///
    /// This function has some overhead, because it needs to take at least a
    /// global read-lock, and potentially a write-lock if the string has not
    /// been seen before. Additionally, opposed to
    /// [`new_static()`](Self::new_static), this function also needs to allocate
    /// a copy of the string on the heap and leak it.
    ///
    /// When the string is statically known at compile time, prefer the
    /// [`sym!(...)`](../stringleton/macro.sym.html) macro. When the string is
    /// statically known to live forever, prefer
    /// [`new_static()`](Self::new_static).
    ///
    /// Please note that symbols are never "garbage collected", so creating an
    /// unbounded number of symbols in this way can be considered a memory leak.
    /// In particular, creating symbols from untrusted user input is a
    /// denial-of-service hazard.
    #[inline]
    #[must_use]
    #[cfg(feature = "alloc")]
    pub fn new(string: impl AsRef<str>) -> Symbol {
        Self::new_(string.as_ref())
    }

    #[inline]
    #[must_use]
    #[cfg(feature = "alloc")]
    fn new_(string: &str) -> Symbol {
        Registry::global().get_or_insert(string)
    }

    /// Create a deduplicated symbol at runtime from a static reference to a
    /// static string.
    ///
    /// If the symbol has not previously been registered, this sidesteps the
    /// need to allocate and leak the string. Using this function does not
    /// allocate memory, outside of what is needed for registering the symbol
    /// for subsequent lookups.
    ///
    /// This function has some overhead, because it needs to take at least a
    /// global read lock, and potentially a write-lock if the string has not
    /// been seen before.
    ///
    /// When the string is statically known at compile time, prefer the
    /// [`sym!(...)`](../stringleton/macro.sym.html) macro.
    ///
    /// The use case for this function is the scenario when a string is only
    /// known at runtime, but the caller wants to allocate it. For example, the
    /// string could be part of a larger (manually leaked) allocation.
    #[inline]
    #[must_use]
    pub fn new_static(string: &'static &'static str) -> Symbol {
        Registry::global().get_or_insert_static(string)
    }

    /// Get a previously registered symbol.
    ///
    /// This returns `None` if the string has not previously been registered.
    ///
    /// This function has some overhead, because it needs to acquire a global
    /// read-lock, but it is faster than [`Symbol::new()`] and never leaks
    /// memory.
    pub fn get(string: impl AsRef<str>) -> Option<Symbol> {
        Self::get_(string.as_ref())
    }

    #[inline]
    fn get_(string: &str) -> Option<Symbol> {
        Registry::global().get(string)
    }

    /// New pre-interned symbol
    ///
    /// # Safety
    ///
    /// `registered_symbol` must be a globally unique string reference (i.e., it
    /// has already been interned through the global registry).
    ///
    /// The only valid external usage of this function is to call it with a
    /// value previously returned from [`Symbol::inner()`].
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(registered_symbol: &'static &'static str) -> Symbol {
        Symbol(registered_symbol)
    }

    /// Get the string representation of this symbol.
    ///
    /// This operation is guaranteed to not take any locks, and is effectively
    /// free.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }

    /// Get the underlying representation of this symbol.
    #[inline]
    #[must_use]
    pub const fn inner(&self) -> &'static &'static str {
        self.0
    }

    /// Get the underlying pointer value of this symbol.
    ///
    /// This is the basis for computing equality and hashes. Symbols
    /// representing the same string always have the same pointer value.
    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> NonNull<&'static str> {
        // SAFETY: Trivial. A static reference cannot be null. This unsafe block
        // can be removed once `#[feature(non_null_from_ref)]` is stabilized.
        unsafe { NonNull::new_unchecked(core::ptr::from_ref::<&'static str>(self.0) as *mut _) }
    }

    /// Convert the symbol to an FFI-friendly `u64`.
    #[inline]
    #[must_use]
    pub fn to_ffi(&self) -> u64 {
        self.as_ptr().as_ptr() as usize as u64
    }

    /// Reconstitute a symbol from a value previously produced by
    /// [`to_ffi()`](Symbol::to_ffi).
    ///
    /// # Safety
    ///
    /// `value` must be produced from a previous call to `to_ffi()` in the
    /// current process, and by the exact same version of this crate.
    ///
    /// In effect, this function can *only* be used for roundtrips through
    /// foreign code.
    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // We don't have 128-bit pointers
    pub unsafe fn from_ffi(value: u64) -> Symbol {
        unsafe { Self::new_unchecked(&*(value as usize as *const &'static str)) }
    }

    /// Reconstitute a symbol from a value previously produced by
    /// [`to_ffi()`](Symbol::to_ffi), checking if it is valid.
    ///
    /// This involves taking a global read-lock to determine the validity of
    /// `value`.
    #[inline]
    #[must_use]
    pub fn try_from_ffi(value: u64) -> Option<Symbol> {
        Registry::global().get_by_address(value)
    }

    /// Length of the underlying string.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether or not this is the empty symbol.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl PartialEq for Symbol {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl Eq for Symbol {}

impl PartialEq<str> for Symbol {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        *self.as_str() == *other
    }
}

impl PartialEq<&str> for Symbol {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        *self.as_str() == **other
    }
}

impl PartialEq<Symbol> for str {
    #[inline]
    fn eq(&self, other: &Symbol) -> bool {
        *self == *other.as_str()
    }
}

impl PartialEq<Symbol> for &str {
    #[inline]
    fn eq(&self, other: &Symbol) -> bool {
        **self == *other.as_str()
    }
}

impl PartialOrd for Symbol {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl PartialOrd<str> for Symbol {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(other))
    }
}

impl PartialOrd<&str> for Symbol {
    #[inline]
    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        Some(self.as_str().cmp(*other))
    }
}

impl PartialOrd<Symbol> for str {
    #[inline]
    fn partial_cmp(&self, other: &Symbol) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other.as_str()))
    }
}

impl PartialOrd<Symbol> for &str {
    #[inline]
    fn partial_cmp(&self, other: &Symbol) -> Option<core::cmp::Ordering> {
        Some((*self).cmp(other.as_str()))
    }
}

impl Hash for Symbol {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

impl AsRef<str> for Symbol {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for Symbol {
    #[inline]
    fn from(value: &str) -> Self {
        Symbol::new(value)
    }
}

#[cfg(feature = "alloc")]
impl From<String> for Symbol {
    #[inline]
    fn from(value: String) -> Self {
        Symbol::new(&*value)
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<Cow<'a, str>> for Symbol {
    fn from(value: Cow<'a, str>) -> Self {
        Symbol::new(&*value)
    }
}

/// Note: This impl forwards string formatting options to the underlying string.
impl core::fmt::Display for Symbol {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

/// Note: This impl forwards string formatting options to the underlying string.
impl core::fmt::Debug for Symbol {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

#[cfg(feature = "serde")]
const _: () = {
    impl serde::Serialize for Symbol {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.as_str().serialize(serializer)
        }
    }

    #[cfg(feature = "alloc")]
    impl<'de> serde::Deserialize<'de> for Symbol {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = Cow::<'de, str>::deserialize(deserializer)?;
            Ok(Symbol::new(&*s))
        }
    }

    #[cfg(not(feature = "alloc"))]
    impl<'de> serde::Deserialize<'de> for Symbol {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = <&'de str>::deserialize(deserializer)?;
            Ok(Symbol::new(&*s))
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "alloc")]
    fn new() {
        let a = Symbol::new("a");
        let b = Symbol::new("b");
        let a2 = Symbol::new("a");
        assert_eq!(a, a2);
        assert_ne!(a, b);
    }

    #[test]
    fn new_static() {
        let a = Symbol::new_static(&"a");
        let b = Symbol::new_static(&"b");
        let a2 = Symbol::new_static(&"a");
        assert_eq!(a, a2);
        assert_ne!(a, b);
    }
}
