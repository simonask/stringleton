use core::{borrow::Borrow, hash::Hash};

use crate::{Site, Symbol};
use hashbrown::{HashMap, hash_map};

#[cfg(feature = "alloc")]
use alloc::{borrow::ToOwned, boxed::Box};

#[cfg(not(any(feature = "std", feature = "critical-section")))]
compile_error!("Either the `std` or `critical-section` feature must be enabled");
#[cfg(not(any(feature = "std", feature = "spin")))]
compile_error!("Either the `std` or `spin` feature must be enabled");

#[cfg(feature = "spin")]
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(feature = "spin"))]
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "critical-section")]
use once_cell::sync::OnceCell as OnceLock;
#[cfg(not(feature = "critical-section"))]
use std::sync::OnceLock;

/// Helper to control the behavior of symbol strings in the registry's hash map.
#[derive(Clone, Copy, PartialEq, Eq)]
struct SymbolStr(&'static &'static str);
impl SymbolStr {
    #[inline]
    fn address(&self) -> usize {
        core::ptr::from_ref::<&'static str>(self.0) as usize
    }
}
impl Borrow<str> for SymbolStr {
    #[inline]
    fn borrow(&self) -> &str {
        self.0
    }
}
impl Hash for SymbolStr {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for SymbolStr {
    #[inline]
    fn from(value: &str) -> Self {
        let value = &*Box::leak(Box::new(&*value.to_owned().leak()));
        Self(value)
    }
}

/// The global symbol registry.
///
/// This is available for advanced use cases, such as bulk-insertion of many
/// symbols.
pub struct Registry {
    #[cfg(not(feature = "spin"))]
    store: std::sync::RwLock<Store>,
    #[cfg(feature = "spin")]
    store: spin::RwLock<Store>,
}

#[derive(Default)]
pub(crate) struct Store {
    by_string: HashMap<SymbolStr, ()>,
    by_pointer: HashMap<usize, SymbolStr>,
}

/// Symbol registry read lock guard
pub struct RegistryReadGuard {
    // Note: Either `std` or `spin`.
    guard: RwLockReadGuard<'static, Store>,
}

/// Symbol registry write lock guard
pub struct RegistryWriteGuard {
    // Note: Either `std` or `spin`.
    guard: RwLockWriteGuard<'static, Store>,
}

impl Registry {
    #[inline]
    fn new() -> Self {
        Self {
            store: RwLock::default(),
        }
    }

    /// Get the global registry.
    pub fn global() -> &'static Registry {
        static REGISTRY: OnceLock<Registry> = OnceLock::new();
        REGISTRY.get_or_init(Registry::new)
    }

    /// Acquire a global read lock of the registry's data.
    ///
    /// New symbols cannot be created while the read lock is held, but acquiring
    /// the lock does not prevent other threads from accessing the string
    /// representation of a [`Symbol`].
    #[inline]
    pub fn read(&'static self) -> RegistryReadGuard {
        RegistryReadGuard {
            #[cfg(not(feature = "spin"))]
            guard: self
                .store
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            #[cfg(feature = "spin")]
            guard: self.store.read(),
        }
    }

    /// Acquire a global write lock of the registry's data.
    ///
    /// Note that acquiring this lock does not prevent other threads from
    /// reading the string representation of a [`Symbol`].
    #[inline]
    pub fn write(&'static self) -> RegistryWriteGuard {
        RegistryWriteGuard {
            #[cfg(not(feature = "spin"))]
            guard: self
                .store
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            #[cfg(feature = "spin")]
            guard: self.store.write(),
        }
    }

    /// Resolve and register symbols from a table.
    ///
    /// You should never need to call this function manually.
    ///
    /// Using the [`stringleton::enable!()`](../stringleton/macro.enable.html)
    /// causes this to be called with the symbols from the current crate in a
    /// static initializer function.
    ///
    /// # Safety
    ///
    /// `table` must not be accessed from any other thread. This is ensured when
    /// this function is called as part of a static initializer function.
    pub unsafe fn register_sites(table: &[Site]) {
        unsafe {
            Registry::global().write().register_sites(table);
        }
    }

    /// Check if the registry contains a symbol matching `string` and return it
    /// if so.
    #[must_use]
    #[inline]
    pub fn get(&'static self, string: &str) -> Option<Symbol> {
        self.read().guard.get(string)
    }

    /// Get the existing symbol for `string`, or insert a new one.
    ///
    /// This opportunistically takes a read lock to check if the symbol exists,
    /// and only takes a write lock if it doesn't.
    ///
    /// If you are inserting many new symbols, prefer acquiring the write lock
    /// by calling [`write()`](Self::write) and then repeatedly call
    /// [`RegistryWriteGuard::get_or_insert()`].
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[must_use]
    pub fn get_or_insert(&'static self, string: &str) -> Symbol {
        let read = self.read();
        if let Some(previously_interned) = read.get(string) {
            return previously_interned;
        }
        core::mem::drop(read);
        let mut write = self.write();
        write.get_or_insert(string)
    }

    /// Get the existing symbol for `string`, or insert a new one.
    ///
    /// This variant is slightly more efficient than
    /// [`get_or_insert()`](Self::get_or_insert), because it can reuse the
    /// storage of `string` directly for this symbol. In other words, if this
    /// call inserted the symbol, the returned [`Symbol`] will be backed by
    /// `string`, and no additional allocations will have happened.
    ///
    /// This opportunistically takes a read lock to check if the symbol exists,
    /// and only takes a write lock if it doesn't.
    ///
    /// If you are inserting many new symbols, prefer acquiring the write lock
    /// by calling [`write()`](Self::write) and then repeatedly call
    /// [`RegistryWriteGuard::get_or_insert_static()`].
    #[inline]
    #[must_use]
    pub fn get_or_insert_static(&'static self, string: &'static &'static str) -> Symbol {
        let read = self.read();
        if let Some(previously_interned) = read.get(string) {
            return previously_interned;
        }
        core::mem::drop(read);

        let mut write = self.write();
        write.get_or_insert_static(string)
    }

    /// Check if a symbol has been registered at `address` (i.e., it has been
    /// produced by [`Symbol::to_ffi()`]), and return the symbol if so.
    ///
    /// This can be used to verify symbols that have made a round-trip over an
    /// FFI boundary.
    #[inline]
    #[must_use]
    pub fn get_by_address(&'static self, address: u64) -> Option<Symbol> {
        self.read().get_by_address(address)
    }
}

impl Store {
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn get_or_insert(&mut self, string: &str) -> Symbol {
        let entry;
        match self.by_string.entry_ref(string) {
            hash_map::EntryRef::Occupied(e) => entry = e,
            hash_map::EntryRef::Vacant(e) => {
                // This calls `SymbolStr::from(string)`, which does the leaking.
                entry = e.insert_entry(());
                let interned = entry.key();
                self.by_pointer.insert(interned.address(), *interned);
            }
        }

        unsafe {
            // SAFETY: We are the registry.
            Symbol::new_unchecked(entry.key().0)
        }
    }

    /// Fast-path for `&'static &'static str` without needing to allocate and
    /// leak some boxes. This is what gets called by the `sym!()` macro.
    pub fn get_or_insert_static(&mut self, string: &'static &'static str) -> Symbol {
        // Caution: Creating a non-interned `SymbolStr` for the purpose of hash
        // table lookup.
        let symstr = SymbolStr(string);

        let interned = match self.by_string.entry(symstr) {
            hash_map::Entry::Occupied(entry) => *entry.key(), // Getting the original key.
            hash_map::Entry::Vacant(entry) => {
                let key = *entry.insert_entry(()).key();
                self.by_pointer.insert(key.address(), key);
                key
            }
        };

        unsafe {
            // SAFETY: We are the registry.
            Symbol::new_unchecked(interned.0)
        }
    }

    pub fn get(&self, string: &str) -> Option<Symbol> {
        self.by_string
            .get_key_value(string)
            .map(|(symstr, ())| unsafe {
                // SAFETY: We are the registry.
                Symbol::new_unchecked(symstr.0)
            })
    }

    #[allow(clippy::cast_possible_truncation)] // We don't have 128-bit pointers
    pub fn get_by_address(&self, address: u64) -> Option<Symbol> {
        self.by_pointer
            .get(&(address as usize))
            .map(|symstr| unsafe {
                // SAFETY: We are the registry.
                Symbol::new_unchecked(symstr.0)
            })
    }
}

impl RegistryReadGuard {
    /// Get the number of registered symbols.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.guard.by_string.len()
    }

    /// Whether or not any symbols are present in the registry.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.guard.by_string.is_empty()
    }

    /// Check if the registry contains a symbol matching `string` and return it
    /// if so.
    ///
    /// This is a simple hash table lookup.
    #[inline]
    #[must_use]
    pub fn get(&self, string: &str) -> Option<Symbol> {
        self.guard.get(string)
    }

    /// Check if a symbol has been registered at `address` (i.e., it has been
    /// produced by [`Symbol::to_ffi()`]), and return the symbol if so.
    ///
    /// This can be used to verify symbols that have made a round-trip over an
    /// FFI boundary.
    #[inline]
    #[must_use]
    pub fn get_by_address(&self, address: u64) -> Option<Symbol> {
        self.guard.get_by_address(address)
    }
}

impl RegistryWriteGuard {
    unsafe fn register_sites(&mut self, sites: &[Site]) {
        unsafe {
            for registration in sites {
                let string = registration.get_string();
                let interned = self.guard.get_or_insert_static(string);
                // Place the interned string pointer at the site and mark it as
                // initialized.
                registration.initialize(interned);
            }
        }
    }

    /// Get the number of registered symbols.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.guard.by_string.len()
    }

    /// Whether or not any symbols are present in the registry.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.guard.by_string.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn get(&self, string: &str) -> Option<Symbol> {
        self.guard.get(string)
    }

    /// Check if a symbol has been registered at `address` (i.e., it has been
    /// produced by [`Symbol::to_ffi()`]), and return the symbol if so.
    ///
    /// This can be used to verify symbols that have made a round-trip over an
    /// FFI boundary.
    #[inline]
    #[must_use]
    pub fn get_by_address(&self, address: u64) -> Option<Symbol> {
        self.guard.get_by_address(address)
    }

    /// Get the existing symbol for `string`, or insert a new one.
    #[inline]
    #[must_use]
    #[cfg(feature = "alloc")]
    pub fn get_or_insert(&mut self, string: &str) -> Symbol {
        self.guard.get_or_insert(string)
    }

    /// Get the existing symbol for `string`, or insert a new one.
    ///
    /// This variant is slightly more efficient than
    /// [`get_or_insert()`](Self::get_or_insert), because it can reuse the
    /// storage of `string` directly for this symbol. In other words, if this
    /// call inserted the symbol, the returned [`Symbol`] will be backed by
    /// `string`, and no additional allocations will have happened.
    #[inline]
    #[must_use]
    pub fn get_or_insert_static(&mut self, string: &'static &'static str) -> Symbol {
        self.guard.get_or_insert_static(string)
    }
}
