//! Switch half: version and site checks, the one inline hook that marks the
//! pop-up as already shown (shared through `share` when another plugin wants
//! the same instruction, chained onto another plugin's hook through `chain`
//! when that plugin does not share), and the plugin API v1 export
//! (`ssbu_quiet_elite_switch_hooked_sites_v1`). The hook runs on the game's UI
//! thread and allocates nothing; the diagnostic build's writer thread logs
//! each suppressed pop-up to `events.log`.

use crate::chain::{self, hook::Outcome, Entry};
use crate::model::{self, Site};
use crate::share;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

const DATA_DIR: &str = "sd:/ultimate/ssbu-quiet-elite-switch";
const OPEN_WRITE_APPEND: i32 = 2 | 4;

/// Counts calls into the heap. Nothing in the plugin is meant to make one.
struct CountingAllocator;
static HEAP_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Set when this plugin installed the hook itself.
static OWNER: AtomicBool = AtomicBool::new(false);
/// `.1a1caa8`: 0 until `main` decides it, then 1 hooked by this plugin, 2 not.
static POPUP_STATE: AtomicU8 = AtomicU8::new(0);
/// Pop-ups suppressed (the flag was clear when the game was about to show one).
static SUPPRESSED: AtomicU64 = AtomicU64::new(0);

fn tick() -> u64 {
    unsafe { nn::os::GetSystemTick() as u64 }
}

/// The pop-up flag read: the one hook, installed only when this plugin owns
/// the site (see `share`), on another plugin's hook there if it found one
/// (see `chain`). Runs this plugin's callback, then every callback other
/// plugins registered for the site.
unsafe extern "C" fn popup(ctx: &mut InlineCtx) {
    quiet_callback(ctx);
    SHARED.for_each(|callback| {
        let callback: unsafe extern "C" fn(*mut InlineCtx) = core::mem::transmute(callback);
        callback(ctx);
    });
}

/// Just before the game reads the pop-up flag: set it, so the game skips the
/// pop-up as if it had shown it this visit. Called from this plugin's hook,
/// or from the site owner's when another plugin owns `.1a1caa8`.
unsafe extern "C" fn quiet_callback(ctx: *mut InlineCtx) {
    let css = (*ctx).registers[20].x();
    if css == 0 {
        return;
    }
    let flag = (css as usize + model::SHOWN_FLAG) as *mut u8;
    if flag.read() == 0 {
        flag.write(1);
        SUPPRESSED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Callbacks other plugins registered for `.1a1caa8` (see `share`).
static SHARED: share::Callbacks = share::Callbacks::new();
static SITE_SHARE: share::SiteShareV1 = share::SiteShareV1 {
    abi_version: share::ABI_VERSION,
    offset: model::POPUP_OFFSET as u64,
    register: register_popup,
};

/// Inline-hook sharing v1 for `.1a1caa8` (see `share`).
#[no_mangle]
pub extern "C" fn ssbu_inline_site_1a1caa8_v1() -> *const share::SiteShareV1 {
    &SITE_SHARE
}

/// Another plugin's callback for `.1a1caa8`; taken when this plugin hooks
/// the site (or will: a registration can come before this `main`, and after
/// it the site holds this plugin's hook, so the answer is `main`'s).
unsafe extern "C" fn register_popup(callback: usize) -> bool {
    let hooks = match POPUP_STATE.load(Ordering::Acquire) {
        0 => version_is_supported() && popup_usable(),
        state => state == 1,
    };
    hooks && SHARED.push(callback)
}

/// Plugin API v1, found by other plugins with `nn::ro::LookupSymbol`: the
/// main-image offsets this plugin has hooked (none unless it owns them).
/// Writes up to `capacity` offsets to `out` and returns how many it hooked.
#[no_mangle]
pub unsafe extern "C" fn ssbu_quiet_elite_switch_hooked_sites_v1(
    out: *mut usize,
    capacity: usize,
) -> usize {
    if !OWNER.load(Ordering::Acquire) {
        return 0;
    }
    if !out.is_null() {
        for (i, site) in model::HOOKED_SITES.iter().take(capacity).enumerate() {
            *out.add(i) = *site;
        }
    }
    model::HOOKED_SITES.len()
}

/// A text line built in a fixed buffer; anything past `N` bytes is dropped.
struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// The text plus a NUL, for `nn::fs` paths; `None` when there is no room.
    fn with_nul(&mut self) -> Option<&[u8]> {
        if self.len >= N {
            return None;
        }
        self.buf[self.len] = 0;
        Some(&self.buf[..=self.len])
    }
}

impl<const N: usize> core::fmt::Write for Line<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

fn path(args: core::fmt::Arguments) -> Line<96> {
    let mut path = Line::new();
    let _ = path.write_fmt(args);
    path
}

/// Appends `bytes` to `events.log`; the file is closed after each write, so
/// it is on the card (and on the Eden host) at once.
fn append_log(bytes: &[u8]) {
    let mut file = path(format_args!("{DATA_DIR}/events.log"));
    let Some(file) = file.with_nul() else {
        return;
    };
    unsafe {
        // Fails harmlessly when the file exists.
        nn::fs::CreateFile(file.as_ptr(), 0);
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, file.as_ptr(), OPEN_WRITE_APPEND) != 0 {
            return;
        }
        let mut offset = 0;
        if nn::fs::GetFileSize(&mut offset, handle) == 0 {
            let option = nn::fs::WriteOption { flags: 0 };
            nn::fs::WriteFile(handle, offset, bytes.as_ptr(), bytes.len() as u64, &option);
            nn::fs::FlushFile(handle);
        }
        nn::fs::CloseFile(handle);
    }
}

/// Eden-only: a writer thread that logs each new suppression.
#[cfg(feature = "diag")]
mod diag {
    use super::*;
    use core::cell::UnsafeCell;

    const WRITER_STACK: usize = 16 * 1024;

    #[repr(C, align(4096))]
    struct ThreadStack(UnsafeCell<[u8; WRITER_STACK]>);
    #[repr(C, align(16))]
    struct ThreadObject(UnsafeCell<[u8; 0x1c0]>);
    // SAFETY: handed to nn::os once, at startup, and never touched again.
    unsafe impl Sync for ThreadStack {}
    unsafe impl Sync for ThreadObject {}
    static WRITER_STACK_MEMORY: ThreadStack = ThreadStack(UnsafeCell::new([0; WRITER_STACK]));
    static WRITER_THREAD: ThreadObject = ThreadObject(UnsafeCell::new([0; 0x1c0]));

    const OPEN_READ: i32 = 1;

    /// `foreign=` in `diag.txt` (see `chain::Foreign`).
    pub fn foreign() -> chain::Foreign {
        let mut file = path(format_args!("{DATA_DIR}/diag.txt"));
        let Some(file) = file.with_nul() else {
            return chain::Foreign::None;
        };
        let mut buf = [0u8; 128];
        let mut read = 0u64;
        unsafe {
            let mut handle = nn::fs::FileHandle { handle: 0 };
            if nn::fs::OpenFile(&mut handle, file.as_ptr(), OPEN_READ) != 0 {
                return chain::Foreign::None;
            }
            let ok =
                nn::fs::ReadFile1(&mut read, handle, 0, buf.as_mut_ptr(), buf.len() as u64) == 0;
            nn::fs::CloseFile(handle);
            if !ok {
                return chain::Foreign::None;
            }
        }
        chain::Foreign::parse(&buf[..(read as usize).min(buf.len())])
    }

    /// Hooks `.1a1caa8` as a plugin loaded before this one would without
    /// sharing (inline, or a far jump).
    pub unsafe fn hook_as_another_plugin(foreign: chain::Foreign) {
        use chain::hook::standin;
        let popup = getRegionAddress(Region::Text) as u64 + model::POPUP_OFFSET as u64;
        match foreign {
            chain::Foreign::Far => standin::far(popup),
            _ => standin::inline(popup, 0),
        }
    }

    extern "C" fn writer_main(_: *mut skyline::libc::c_void) {
        let mut logged = 0;
        let mut standins = [0; 3];
        loop {
            unsafe {
                nn::os::SleepThread(nn::TimeSpan {
                    nanoseconds: 200_000_000,
                })
            };
            let now = SUPPRESSED.load(Ordering::Relaxed);
            if now != logged {
                logged = now;
                let mut line = Line::<96>::new();
                let _ = writeln!(line, "{} diag suppressed total={now}", tick());
                append_log(line.as_bytes());
            }
            let hits = chain::hook::standin::HITS.each_ref().map(|h| h.load(Ordering::Relaxed));
            if hits != standins {
                standins = hits;
                let mut line = Line::<96>::new();
                let _ = writeln!(
                    line,
                    "{} diag standin popup_inline={} popup_far={}",
                    tick(),
                    hits[0],
                    hits[2]
                );
                append_log(line.as_bytes());
            }
        }
    }

    pub fn start_writer() -> bool {
        unsafe {
            let thread = WRITER_THREAD.0.get().cast::<nn::os::ThreadType>();
            let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
            if nn::os::CreateThread(
                thread,
                writer_main,
                core::ptr::null_mut(),
                WRITER_STACK_MEMORY.0.get().cast::<u8>(),
                WRITER_STACK,
                priority,
            ) != 0
            {
                return false;
            }
            nn::os::SetThreadNamePointer(thread, c"quiet-elite-switch".as_ptr().cast());
            nn::os::StartThread(thread);
            true
        }
    }
}

/// The site's address, when its 64 bytes are in `.text`.
unsafe fn site_address(site: &Site) -> Option<u64> {
    let base = getRegionAddress(Region::Text) as u64;
    let end = getRegionAddress(Region::Rodata) as u64;
    let address = base + site.offset as u64;
    (address.is_multiple_of(4) && address + 64 <= end).then_some(address)
}

/// Whether the hook can go on `.1a1caa8` as it is now (see `chain::entry`).
fn popup_usable() -> bool {
    let site = &model::POPUP;
    unsafe { site_address(site) }.is_some_and(|address| {
        let words = unsafe { chain::hook::words_at(address) };
        chain::entry(&site.stock(), &words, address) != Entry::Foreign
    })
}

/// Hooks `.1a1caa8`, on another plugin's hook there if it finds one.
unsafe fn hook_popup() -> (Entry, Outcome) {
    let site = &model::POPUP;
    match site_address(site) {
        Some(address) => chain::hook::inline(address, &site.stock(), popup),
        None => (Entry::Foreign, Outcome::Skipped),
    }
}

fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    version.name[..end] == *model::SUPPORTED_VERSION.as_bytes()
}

/// The `SiteShareV1` table whoever's `ssbu_inline_site_1a1caa8_v1` resolves
/// first returns (0 when no plugin exports it).
fn site_owner_table() -> usize {
    let mut export = 0usize;
    unsafe { nn::ro::LookupSymbol(&mut export, b"ssbu_inline_site_1a1caa8_v1\0".as_ptr()) };
    if export == 0 {
        return 0;
    }
    let export: extern "C" fn() -> *const share::SiteShareV1 =
        unsafe { core::mem::transmute(export) };
    export() as usize
}

/// Appends the install line to `events.log` and the Skyline log.
fn report_install(report: &mut Line<256>) {
    let _ = write!(
        report,
        " heap_allocations={}",
        HEAP_ALLOCATIONS.load(Ordering::Relaxed)
    );
    skyline::logging::log(core::str::from_utf8(report.as_bytes()).unwrap_or(""));
    let _ = report.write_char('\n');
    for directory in ["sd:/ultimate", DATA_DIR] {
        if let Some(directory) = path(format_args!("{directory}")).with_nul() {
            unsafe { nn::fs::CreateDirectory(directory.as_ptr()) };
        }
    }
    append_log(report.as_bytes());
}

#[skyline::main(name = "ssbu-quiet-elite-switch")]
pub fn main() {
    let mut report = Line::<256>::new();
    let _ = write!(
        report,
        "{} plugin=ssbu-quiet-elite-switch/{}",
        tick(),
        env!("CARGO_PKG_VERSION")
    );
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    #[cfg(feature = "diag")]
    {
        let foreign = diag::foreign();
        if foreign != chain::Foreign::None {
            unsafe { diag::hook_as_another_plugin(foreign) };
            let _ = write!(report, " foreign={}", foreign.name());
        }
    }
    // Election first (see `share`): when another plugin owns `.1a1caa8` its
    // hook is already there, so only the owner reads the site. One hook per
    // instruction: the owner installs it, the others run from it.
    let own_table = &SITE_SHARE as *const share::SiteShareV1 as usize;
    let result = match share::role(site_owner_table(), own_table) {
        share::Role::Owner => {
            let (entry, outcome) = unsafe { hook_popup() };
            let hooked = outcome.hooked();
            POPUP_STATE.store(if hooked { 1 } else { 2 }, Ordering::Release);
            OWNER.store(hooked, Ordering::Release);
            let _ = write!(
                report,
                " role=owner popup={}/{} guests={}",
                entry.name(),
                outcome.name(),
                SHARED.len()
            );
            if hooked {
                "installed"
            } else {
                "not_installed"
            }
        }
        share::Role::Guest(table) => {
            POPUP_STATE.store(2, Ordering::Release);
            let _ = write!(report, " role=guest owner={table:#x}");
            let api = unsafe { &*(table as *const share::SiteShareV1) };
            if share::compatible(api, model::POPUP_OFFSET)
                && unsafe { (api.register)(quiet_callback as *const () as usize) }
            {
                "installed"
            } else {
                "owner_refused"
            }
        }
    };
    let _ = write!(report, " result={result}");
    report_install(&mut report);
    #[cfg(feature = "diag")]
    if !diag::start_writer() {
        report = Line::new();
        let _ = write!(report, "{} writer=not_started", tick());
        report_install(&mut report);
    }
}
