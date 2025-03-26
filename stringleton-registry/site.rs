use core::{cell::UnsafeCell, pin::Pin};

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
    #[cfg(any(miri, feature = "debug-assertions"))]
    initialized: core::sync::atomic::AtomicBool,
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
            #[cfg(any(miri, feature = "debug-assertions"))]
            initialized: core::sync::atomic::AtomicBool::new(false),
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
        #[cfg(any(miri, feature = "debug-assertions"))]
        {
            self.initialized
                .store(true, core::sync::atomic::Ordering::SeqCst);
        }
        unsafe {
            *self.inner.get() = interned.inner();
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
    pub unsafe fn get_after_ctor(self: Pin<&'static Self>) -> Symbol {
        #[cfg(miri)]
        unsafe {
            return get_without_ctor_support(self);
        }

        #[cfg(not(miri))]
        unsafe {
            get_with_ctor_support(self)
        }
    }
}

/// # Safety
///
/// Must be called after static ctors have run.
#[inline(always)]
unsafe fn get_with_ctor_support(site: Pin<&'static Site>) -> Symbol {
    #[cfg(feature = "debug-assertions")]
    {
        assert!(
            site.initialized.load(core::sync::atomic::Ordering::Relaxed),
            "This `sym!()` call site has not been initialized by a static constructor. This can happen for two reasons: \n
  a) The current platform does not support static constructors (e.g., Miri)\n
  b) The current crate is a dynamic library, but it reuses the registration from another crate, i.e., stringleton!(foreign_crate) is being used across a dynamic linking boundary"
            );
    }

    unsafe {
        // SAFETY: The safety invariant is that this is only called after ctors
        // have run, and only ctors write to this location.
        Symbol::new_unchecked(*site.inner.get())
    }
}

#[inline(always)]
#[cfg(miri)]
unsafe fn get_without_ctor_support(site: Pin<&'static Site>) -> Symbol {
    use core::sync::atomic::{AtomicPtr, Ordering};

    let mut atomic_ptr;
    let atomic_inner: &AtomicPtr<&'static &'static str> = unsafe {
        atomic_ptr = site.inner.get();
        // SAFETY: `inner` is only ever accessed atomically, and the pointer
        // is valid.
        AtomicPtr::from_ptr(&mut atomic_ptr)
    };

    if site.initialized.load(Ordering::SeqCst) {
        unsafe {
            // SAFETY: Already initialized, safe to use the inner pointer
            // directly.
            return Symbol::new_unchecked(*(atomic_inner.load(Ordering::SeqCst)));
        }
    }

    let stored_value = unsafe {
        // SAFETY: The pointer is valid.
        //
        // RELAXED: It doesn't matter if we read an outdated value here, because
        // `initialized` is what controls the order of operations.
        &*(atomic_inner.load(Ordering::Relaxed))
    };

    let interned = crate::Registry::global().get_or_insert_static(stored_value);

    // Store the value. Note: This is idempotent, because
    // `Registry::get_or_insert_static()` is guaranteed to return the same
    // pointer for the same string value.
    atomic_inner.store(
        core::ptr::from_ref::<&'static str>(interned.inner()) as *mut _,
        Ordering::SeqCst,
    );
    // Use the fast path for subsequent calls.
    site.initialized.store(true, Ordering::SeqCst);
    interned
}
