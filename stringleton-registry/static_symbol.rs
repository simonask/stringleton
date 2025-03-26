use crate::{Site, Symbol};

/// Const-compatible static symbol.
///
/// This type is created by the
/// [`static_sym!(...)`](../stringleton/macro.static_sym.html) macro, and can be
/// used in const contexts. See the macro documentation for more details.
///
/// **CAUTION:** Declarations with `StaticSymbol` must _not_ be used before
/// static initializers have run, i.e. before `main()`.
#[derive(Copy, Clone)]
pub struct StaticSymbol(
    // Note: Unfortunately, we can't use a `&'static Site` reference directly,
    // because it messes with a (possibly overzealous?) UB check in the
    // compiler, probably because `Site` contains an `UnsafeCell`? Going through
    // a function pointer sidesteps this issue, at very slightly higher cost,
    // due to the extra indirection.
    //
    // This function pointer is always a simple trampoline that simply returns a
    // static reference into the `.bss` segment.
    fn() -> &'static Site,
);

impl StaticSymbol {
    /// # Safety
    ///
    /// Must only be called when the site returned by `f` participates in the
    /// static table of symbols that will be initialized by a static initializer
    /// function. This invariant is ensured by the `static_sym!(...)` macro.
    #[must_use]
    #[doc(hidden)]
    pub const unsafe fn new_unchecked(f: fn() -> &'static Site) -> Self {
        Self(f)
    }
}

impl core::ops::Deref for StaticSymbol {
    type Target = Symbol;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: Precondition for `StaticSymbol` is that it must only be
            // used after static initializers have run.
            self.0().get_ref_after_ctor()
        }
    }
}

impl core::borrow::Borrow<Symbol> for StaticSymbol {
    #[inline]
    fn borrow(&self) -> &Symbol {
        self
    }
}

impl AsRef<Symbol> for StaticSymbol {
    #[inline]
    fn as_ref(&self) -> &Symbol {
        self
    }
}

impl From<&StaticSymbol> for Symbol {
    #[inline]
    fn from(value: &StaticSymbol) -> Self {
        **value
    }
}

impl From<StaticSymbol> for Symbol {
    #[inline]
    fn from(value: StaticSymbol) -> Self {
        *value
    }
}

impl PartialEq<Symbol> for StaticSymbol {
    #[inline]
    fn eq(&self, other: &Symbol) -> bool {
        **self == *other
    }
}

impl PartialEq<StaticSymbol> for Symbol {
    #[inline]
    fn eq(&self, other: &StaticSymbol) -> bool {
        *self == **other
    }
}

impl PartialEq for StaticSymbol {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl core::fmt::Debug for StaticSymbol {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl core::fmt::Display for StaticSymbol {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
    }
}
