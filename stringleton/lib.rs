#![doc = include_str!("../README.md")]

pub use stringleton_registry::{Registry, StaticSymbol, Symbol};

/// Create a literal symbol from a literal identifier or string
///
/// Symbols created with the [`sym!(...)`](sym) macro are statically allocated
/// and deduplicated on program startup. This means that there is no discernible
/// overhead at the point of use, making them suitable even in long chains of
/// `if` statements and inner loops.
///
/// **IMPORTANT:** For this macro to work in a particular crate, the
/// [`enable!()`](crate::enable) macro must appear exactly once in the crate's
/// root. This creates the global registration table at link-time.
///
/// # Safety
///
/// This macro is safe (and performant) to use everywhere, with important
/// caveats:
///
/// 1. If you are using "static initializers" (code that runs before `main()`,
///    like through the `ctor` crate), this macro must **NOT** be called in such
///    a static initializer function. See
///    <https://github.com/mmastrac/rust-ctor/issues/159>. Using
///    [`Symbol::new()`] in such a function is fine.
///
/// 2. If you are using C-style dynamic libraries (`cdylib` crate type), those
///    libraries must use the `stringleton-dylib` crate instead of
///    `stringleton`.
///
/// 3. If you are loading dynamic libraries at runtime (i.e., outside of Cargo's
///    dependency graph), the host crate must also use the `stringleton-dylib`
///    crate instead of `stringleton`.
///
/// # Low-level details
///
/// This macro creates an entry in a per-crate `linkme` "distributed slice", as
/// well as a static initializer called by the OS when the current crate is
/// loaded at runtime (before `main()`), either as part of an executable or as
/// part of a dynamic library.
///
/// On x86-64 and ARM64, this macro is guaranteed to compile into a single
/// relaxed atomic memory load instruction from an offset in the `.bss` segment.
/// On x86, relaxed atomic load instructions have no additional overhead
/// compared to non-atomic loads.
///
/// Internally, this uses the `linkme` and `ctor` crates to register this
/// callsite in static binary memory and initialize it on startup. However, when
/// running under Miri (or other platforms not supported by `linkme`), the
/// implementation falls back on a slower implementation that effectively calls
/// `Symbol::new()` every time, which takes a global read-lock.
///
/// When the `debug-assertions` feature is enabled, there is an additional check
/// that panics if the call site has not been populated by a static ctor. This
/// assertion will only be triggered if the current platform does not support
/// static initializers.
#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! sym {
    ($sym:ident) => {
        $crate::sym!(@impl stringify!($sym))
    };
    ($sym:literal) => {
        $crate::sym!(@impl $sym)
    };
    (@impl $sym:expr) => {{
        // Note: Using `crate` to refer to the calling crate - this is deliberate.
        #[$crate::internal::linkme::distributed_slice(crate::_stringleton_enabled::TABLE)]
        #[linkme(crate = $crate::internal::linkme)]
        static SITE: $crate::internal::Site = $crate::internal::Site::new(&$sym);
        unsafe {
            // SAFETY: This site will be initialized by the static ctor because
            // it participates in the distributed slice.
            SITE.get_after_ctor()
        }}
    }
}

/// Create a static location for a literal symbol.
///
/// This macro works the same as [`sym!(...)`](sym), except that it produces a
/// [`StaticSymbol`] instead of a [`Symbol`]. [`StaticSymbol`] implements
/// `Deref<Target = Symbol>`, so it can be used in most places where a `Symbol`
/// is expected.
///
/// This macro also requires the presence of a call to the [`enable!()`](enable)
/// macro at the crate root.
///
/// This macro can be used in the initialization of a `static` or `const` variable:
///
/// ```rust,ignore
/// static MY_SYMBOL: StaticSymbol = static_sym!("Hello, World!");
/// const OTHER_SYMBOL: StaticSymbol = static_sym!(abc);
///
/// assert_eq!(MY_SYMBOL, sym!("Hello, World!"));
/// assert_eq!(OTHER_SYMBOL, sym("abc"));
/// ```
///
/// # Use case
///
/// Use this macro to avoid having too many "magic symbols" in your code
/// (similar to "magic numbers"). Declare common symbol names centrally, and
/// refer to them by their Rust names instead.
///
/// At runtime, using symbols declared as `static_sym!(...)` is actually very
/// slightly less efficient than using `sym!(...)` directly, due to a necessary
/// extra indirection. This is probably negligible in almost all cases, but it
/// is counterintuitive nevertheless. _(This caveat may be lifted in future, but
/// is due to a - potentially overzealous - check in the compiler which requires
/// the indirection.)_
///
/// # Low-level details
///
/// Another (extremely niche) effect of using this macro over `sym!(...)` is
/// that it can help reduce the link-time size of the symbol table. Each
/// `sym!(...)` and `static_sym!(...)` call site adds 8 bytes to the `.bss`
/// segment, so this can only matter when you have in the order of millions of
/// symbols in your binary. Still, worth knowing if you are golfing binary size.
#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! static_sym {
    ($sym:ident) => {
        $crate::static_sym!(@impl stringify!($sym))
    };
    ($sym:literal) => {
        $crate::static_sym!(@impl $sym)
    };
    (@impl $sym:expr) => {{
        unsafe {
            // SAFETY: `new_unchecked()` is called with a `Site` that
            // participates in the crate's symbol table.
            $crate::StaticSymbol::new_unchecked({
                // Tiny function just to get the `Site` for this symbol.
                fn _stringleton_static_symbol_call_site() -> &'static $crate::internal::Site {
                    // Note: Using `crate` to refer to the calling crate - this is deliberate.
                    #[$crate::internal::linkme::distributed_slice(crate::_stringleton_enabled::TABLE)]
                    #[linkme(crate = $crate::internal::linkme)]
                    static SITE: $crate::internal::Site = $crate::internal::Site::new(&$sym);
                    &SITE
                }
                _stringleton_static_symbol_call_site
            })
        }
    }}
}

/// Enable the [`sym!(...)`](sym) macro in the calling crate.
///
/// Put a call to this macro somewhere in the root of each crate that uses the
/// `sym!(...)` macro.
///
/// ## Details
///
/// This creates a "distributed slice" containing all symbols in this crate, as
/// well as a static constructor that deduplicates all symbols on startup, or
/// when a dynamic library is loaded when the target binary is a `dylib` or a
/// `cdylib`.
///
/// This macro may also be invoked with a module path to another crate, which
/// causes symbols in this crate to be registered as part of symbols in the
/// other crate.
///
/// **CAUTION:** Using the second variant is discouraged, because it will not
/// work when the other crate is being loaded as a dynamic library. However, it
/// is very slightly more efficient.
#[macro_export]
macro_rules! enable {
    () => {
        #[doc(hidden)]
        pub(crate) mod _stringleton_enabled {
            #[$crate::internal::linkme::distributed_slice]
            #[linkme(crate = $crate::internal::linkme)]
            #[doc(hidden)]
            pub(crate) static TABLE: [$crate::internal::Site] = [..];

            $crate::internal::ctor::declarative::ctor! {
                #[ctor]
                #[doc(hidden)]
                pub fn _stringleton_register_symbols() {
                    unsafe {
                        // SAFETY: This is a static ctor.
                        $crate::internal::Registry::register_sites(&TABLE);
                    }
                }
            }
        }

        #[allow(unused)]
        #[doc(hidden)]
        pub use _stringleton_enabled::_stringleton_register_symbols;
    };
    ($krate:path) => {
        #[doc(hidden)]
        pub(crate) use $krate::_stringleton_enabled;
    };
}

#[doc(hidden)]
pub mod internal {
    pub use ctor;
    pub use linkme;
    pub use stringleton_registry::Registry;
    pub use stringleton_registry::Site;
}
