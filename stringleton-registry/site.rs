#[allow(unused_imports)]
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use crate::Symbol;

/// Registration site for a static symbol created by the `sym!()` macro in
/// `stringleton`.
///
/// You should never need to construct this manually.
#[repr(C)]
pub struct Site {
    /// Before global symbol registration, this is the string that will be interned. After global
    /// symbol registration, this contains the value of the symbol directly.
    ///
    /// Safety: Access to this field is guarded in different ways at different points in time.
    ///
    /// - Static initializer functions are guaranteed to run in sequence (no
    ///   threads), so access is trivially synchronized.
    /// - After static initializers, this field is only ever read immutably.
    inner: UnsafeCell<&'static &'static str>,
    #[cfg(any(miri, target_arch = "wasm32", feature = "debug-assertions"))]
    initialized: AtomicBool,
}

// SAFETY: The contents of `SymbolRegistration` are synchronized by (a) static
// constructors at upstart, or (b) a global rwlock at runtime.
//
// Note that `SymbolRegistrationSite` does not need to (and probably should not)
// implement `Send`, only `Sync`.
unsafe impl Sync for Site {}

impl Site {
    #[inline(always)]
    #[must_use]
    #[doc(hidden)]
    pub const fn new(string: &'static &'static str) -> Self {
        Self {
            inner: UnsafeCell::new(string),
            #[cfg(any(miri, target_arch = "wasm32", feature = "debug-assertions"))]
            initialized: AtomicBool::new(false),
        }
    }

    /// # Safety
    ///
    /// This must only be called from the registry's static ctor, or after
    /// static ctors have finished running.
    #[inline(always)]
    pub unsafe fn get_string(&self) -> &'static &'static str {
        unsafe {
            // SAFETY: Preconditions of `initialize`.
            *self.inner.get()
        }
    }

    /// Initialize the symbol callsite.
    ///
    /// # Safety
    ///
    /// This must only be called from static constructors.
    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn initialize(&self, interned: Symbol) {
        #[cfg(any(miri, target_arch = "wasm32", feature = "debug-assertions"))]
        {
            self.initialized
                .store(true, core::sync::atomic::Ordering::SeqCst);
        }
        unsafe {
            *self.inner.get() = interned.inner();
        }
    }

    /// Get a reference to the symbol contained in this site.
    ///
    /// # Safety
    ///
    /// This *MUST* only be called when `self` is part of the distributed slice
    /// used by the ctor, and after static ctors have run. For example,
    /// obtaining a `&'static self` via `Box::leak()` and calling this will not
    /// work.
    #[inline(always)]
    #[must_use]
    pub unsafe fn get_ref_after_ctor(&'static self) -> &'static Symbol {
        #[cfg(any(miri, target_arch = "wasm32"))]
        unsafe {
            // Slow path.
            return get_without_ctor_support(self);
        }

        #[cfg(not(any(miri, target_arch = "wasm32")))]
        unsafe {
            // Fast path.
            get_with_ctor_support(self)
        }
    }

    /// Get the deduplicated symbol value.
    ///
    /// # Safety
    ///
    /// This *MUST* only be called when `self` is part of the distributed slice
    /// used by the ctor, and after static ctors have run. For example,
    /// obtaining a `&'static self` via `Box::leak()` and calling this will not
    /// work.
    #[inline(always)]
    #[must_use]
    pub unsafe fn get_after_ctor(&'static self) -> Symbol {
        unsafe { *self.get_ref_after_ctor() }
    }
}

/// # Safety
///
/// Must be called after static ctors have run.
#[inline(always)]
#[allow(unused)] // unused under `cfg(any(miri, target_arch = "wasm32"))`
unsafe fn get_with_ctor_support(site: &'static Site) -> &'static Symbol {
    #[cfg(feature = "debug-assertions")]
    {
        assert!(
            site.initialized.load(core::sync::atomic::Ordering::Relaxed),
            "This `sym!()` call site has not been initialized by a static constructor. This can happen for the following reasons: \n
  a) The current platform does not support static constructors (e.g., Miri)\n
  b) The current crate is a dynamic library, but it reuses the registration from another crate, i.e., stringleton!(foreign_crate) is being used across a dynamic linking boundary\n
  c) The call site is somehow reached without its containing binary having its static ctor functions called"
            );
    }

    unsafe {
        // SAFETY: The safety invariant is that this is only called after ctors
        //         have run, and only ctors write to this location, so we can do
        //         a non-atomic load.
        let ptr: *const &'static &'static str = site.inner.get();
        // SAFETY: Symbol is `#[repr(transparent)]`, so it is safe to cast
        //         `&&'static &'static str` to `&Symbol`.
        let ptr: *const Symbol = ptr.cast();
        &*ptr
    }
}

/// This is the "slow path" used when Miri is active, because `linkme` and
/// `ctor` are not supported there. It performs an atomic check on every access,
/// and is therefore a lot slower.
#[inline(always)]
#[cfg(any(miri, target_arch = "wasm32"))]
unsafe fn get_without_ctor_support(site: &'static Site) -> &'static Symbol {
    // CAUTION:
    //
    // Hold on for dear life, things are about to get nasty.

    // This performs no memory access, only pointer casts.
    let inner_ptr: *mut *mut &'static str = {
        // We're casting a `&'static &'static str` to a `*mut &'static str`, and
        // it's fine because we are never actually writing through the second
        // indirection.
        let ptr: *mut &'static &'static str = site.inner.get();
        ptr.cast()
    };

    if site.initialized.load(Ordering::SeqCst) {
        unsafe {
            // SAFETY:
            // - Already initialized, so it is safe to access `inner`
            //   non-atomically.
            // - Symbol is `repr(transparent)`, so it is safe to cast a
            //   `&&'static &'static str` to `&'static Symbol`.
            return &*(inner_ptr as *const Symbol);
        }
    }

    unsafe {
        // SAFETY: See `initialize_atomic`.
        initialize_atomic(inner_ptr, &site.initialized);
    }

    unsafe {
        // SAFETY: Non-atomic access is safe from here on out.
        &*(inner_ptr as *const Symbol)
    }
}

#[cfg(any(miri, target_arch = "wasm32"))]
unsafe fn initialize_atomic(inner_ptr: *mut *mut &'static str, initialized: &'static AtomicBool) {
    // Cast to an atomic pointer
    let atomic_inner: &AtomicPtr<&'static str> = unsafe {
        // SAFETY: Until we set `initialized = true`, the location is only
        // accessed atomically.
        AtomicPtr::from_ptr(inner_ptr)
    };

    let stored_value: &'static &'static str = unsafe {
        // SAFETY: The pointer is valid.
        //
        // RELAXED: It doesn't matter if we read an outdated value here, because
        // `initialized` is what controls the order of operations, and we
        // unconditionally perform a `SeqCst` load above and one below.
        &*(atomic_inner.load(Ordering::Relaxed))
    };

    let interned = crate::Registry::global().get_or_insert_static(stored_value);

    // Store the value.
    //
    // Note: This is idempotent, because `Registry::get_or_insert_static()` is
    // guaranteed to return the same pointer for the same string value.
    let ptr = core::ptr::from_ref(interned.inner());
    atomic_inner.store(ptr as *mut &'static str, Ordering::SeqCst);

    // Use the fast path for subsequent calls. Nobody takes the non-atomic route
    // until they see this store.
    initialized.store(true, Ordering::SeqCst);
}
