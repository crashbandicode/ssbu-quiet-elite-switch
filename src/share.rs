//! Inline-hook sharing, v1 (the same file in gsp-visualizer and
//! ssbu-quiet-elite-switch).
//!
//! A plugin that wants to inline-hook main-image offset `X` exports
//! `ssbu_inline_site_<X in lower-case hex>_v1() -> *const SiteShareV1` and,
//! before hooking, looks that name up with `nn::ro::LookupSymbol`. Skyline
//! loads every plugin before any `main` runs, so every plugin gets the same
//! answer whatever the load order:
//!
//! - the answer is its own export: it owns the site, installs the one hook and
//!   calls every registered callback from it, after its own;
//! - the answer is another plugin's export: it registers its callback there
//!   and installs nothing.
//!
//! So two plugins that want the same instruction never patch it twice (two
//! inline hooks on one instruction collide), and both keep working. The
//! callback receives the hook's `*mut InlineCtx` (Skyline 0.6 layout) and may
//! read or change registers as a hook would.

use core::sync::atomic::{AtomicUsize, Ordering};

pub const ABI_VERSION: u32 = 1;
pub const MAX_CALLBACKS: usize = 8;

/// What `ssbu_inline_site_<X>_v1` returns.
#[repr(C)]
pub struct SiteShareV1 {
    pub abi_version: u32,
    /// The main-image offset the site is.
    pub offset: u64,
    /// Adds a callback (`unsafe extern "C" fn(*mut InlineCtx)`, as an
    /// address). False when the owner will not hook the site (another game
    /// version, or the site's bytes changed) or its table is full; the caller
    /// then installs nothing either.
    pub register: unsafe extern "C" fn(callback: usize) -> bool,
}

/// Registered callback addresses, in registration order. Filled while the
/// plugins' `main`s run, read from the hook afterwards.
pub struct Callbacks {
    slots: [AtomicUsize; MAX_CALLBACKS],
    len: AtomicUsize,
}

impl Callbacks {
    pub const fn new() -> Self {
        Self {
            slots: [const { AtomicUsize::new(0) }; MAX_CALLBACKS],
            len: AtomicUsize::new(0),
        }
    }

    /// Adds `callback`; false for a null address or a full table.
    pub fn push(&self, callback: usize) -> bool {
        if callback == 0 {
            return false;
        }
        let mut len = self.len.load(Ordering::Acquire);
        loop {
            if len >= MAX_CALLBACKS {
                return false;
            }
            match self
                .len
                .compare_exchange(len, len + 1, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    self.slots[len].store(callback, Ordering::Release);
                    return true;
                }
                Err(now) => len = now,
            }
        }
    }

    /// Calls `f` with each registered address, in order. A slot reserved but
    /// not yet written is skipped.
    pub fn for_each(&self, mut f: impl FnMut(usize)) {
        let len = self.len.load(Ordering::Acquire).min(MAX_CALLBACKS);
        for slot in &self.slots[..len] {
            let callback = slot.load(Ordering::Acquire);
            if callback != 0 {
                f(callback);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire).min(MAX_CALLBACKS)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Callbacks {
    fn default() -> Self {
        Self::new()
    }
}

/// Who handles a site, from the table the looked-up export returned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The name resolves to this plugin's own table (or to nothing).
    Owner,
    /// It resolves to another plugin's table at this address.
    Guest(usize),
}

/// `found_table` is what the export `LookupSymbol` returned gave back (0 when
/// the name did not resolve); `own_table` is this plugin's `SiteShareV1`.
pub fn role(found_table: usize, own_table: usize) -> Role {
    if found_table == 0 || found_table == own_table {
        Role::Owner
    } else {
        Role::Guest(found_table)
    }
}

/// Whether an owner's table speaks this version for this site.
pub fn compatible(api: &SiteShareV1, offset: usize) -> bool {
    api.abi_version == ABI_VERSION && api.offset == offset as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callbacks_keep_order_and_cap() {
        let table = Callbacks::new();
        assert!(table.is_empty());
        assert!(!table.push(0));
        for i in 1..=MAX_CALLBACKS {
            assert!(table.push(i * 0x10));
        }
        assert!(!table.push(0x999));
        let mut seen = Vec::new();
        table.for_each(|c| seen.push(c));
        assert_eq!(
            seen,
            (1..=MAX_CALLBACKS).map(|i| i * 0x10).collect::<Vec<_>>()
        );
        assert_eq!(table.len(), MAX_CALLBACKS);
    }

    #[test]
    fn role_follows_the_lookup() {
        assert_eq!(role(0, 0x1000), Role::Owner);
        assert_eq!(role(0x1000, 0x1000), Role::Owner);
        assert_eq!(role(0x2000, 0x1000), Role::Guest(0x2000));
    }

    #[test]
    fn compatibility_needs_version_and_offset() {
        unsafe extern "C" fn never(_: usize) -> bool {
            false
        }
        let api = SiteShareV1 {
            abi_version: ABI_VERSION,
            offset: 0x1a1c118,
            register: never,
        };
        assert!(compatible(&api, 0x1a1c118));
        assert!(!compatible(&api, 0x1a1caa8));
        let old = SiteShareV1 {
            abi_version: 0,
            ..api
        };
        assert!(!compatible(&old, 0x1a1c118));
    }
}
