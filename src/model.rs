//! Host-testable half: the hook site and its guard.
//!
//! When the selected fighter's Elite Smash status changes on the Quickplay
//! character select, `.1a1c9f0(css, _, is_elite)` switches the screen theme.
//! Switching away from an Elite fighter it plays `black_base_def` and
//! `title_on`, then, unless the CSS flag `+0xf8e` is set, parses and shows
//! the pop-up record `0x58ffff206396e7a9` (`.32edf70`, `.32f1bb0`) and sets the
//! flag, so the pop-up appears once per visit. `+0xf8e` is read only at
//! `.1a1caa8` and written only at `.1a1cb0c`. The plugin sets it just before
//! that read; the theme switch is unchanged.

pub const SUPPORTED_VERSION: &str = "13.0.5";

/// A hooked site and the 64 bytes that must be there (13.0.5 image). Another
/// plugin's hook here means this plugin installs nothing.
pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub guard: [u8; 64],
}

/// `ldrb w8, [x20, #0xf8e]`, the pop-up flag read; x20 = the CSS object.
/// Nothing jumps into the 16 bytes from here.
pub const POPUP_OFFSET: usize = 0x1a1caa8;
pub const POPUP: Site = Site {
    name: "popup",
    offset: POPUP_OFFSET,
    guard: [
        0x88, 0x3a, 0x7e, 0x39, 0xc8, 0x02, 0x00, 0x35, 0x88, 0xc8, 0x01, 0xf0, 0x16, 0x11, 0x44,
        0xf9, 0x88, 0xc8, 0x01, 0xf0, 0x08, 0x99, 0x43, 0xf9, 0x08, 0x05, 0x40, 0xf9, 0x00, 0xe5,
        0x40, 0xf9, 0x35, 0xf5, 0x9c, 0xd2, 0xd5, 0x72, 0xac, 0xf2, 0x15, 0xe4, 0xdf, 0xf2, 0xf5,
        0x1f, 0xeb, 0xf2, 0xe1, 0x03, 0x15, 0xaa, 0x9d, 0xd1, 0x61, 0x94, 0xe0, 0x00, 0x00, 0x36,
        0xc0, 0x0a, 0x40, 0xf9,
    ],
};

/// The CSS object's "pop-up already shown this visit" flag.
pub const SHOWN_FLAG: usize = 0xf8e;

/// The pop-up record the game shows there.
pub const POPUP_RECORD: u64 = 0x58ff_ff20_6396_e7a9;

/// Every site checked before anything is installed.
pub const SITES: [&Site; 1] = [&POPUP];

/// The main-image offsets this plugin hooks, as
/// `ssbu_quiet_elite_switch_hooked_sites_v1` reports them to other plugins.
pub const HOOKED_SITES: [usize; 1] = [POPUP_OFFSET];

/// The 64-bit value a MOVZ + three MOVK (hw 1..3) build in one register.
pub fn movz_movk(words: [u32; 4]) -> Option<u64> {
    let rd = words[0] & 31;
    if words[0] & 0xffe0_0000 != 0xd280_0000 {
        return None;
    }
    let mut value = u64::from((words[0] >> 5) & 0xffff);
    for (hw, word) in words[1..].iter().enumerate() {
        if word & 0xff80_0000 != 0xf280_0000 || (word >> 21) & 3 != hw as u32 + 1 || word & 31 != rd
        {
            return None;
        }
        value |= u64::from((word >> 5) & 0xffff) << (16 * (hw + 1));
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(at: usize) -> u32 {
        u32::from_le_bytes(POPUP.guard[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn site_reads_the_shown_flag_of_the_css_object() {
        let w = word(0);
        assert_eq!(w & 0xffc0_0000, 0x3940_0000, "ldrb (unsigned offset)");
        assert_eq!((w >> 10) & 0xfff, SHOWN_FLAG as u32);
        assert_eq!((w >> 5) & 31, 20, "x20 = the CSS object");
        assert_eq!(w & 31, 8, "w8");
        // cbnz w8 skips the pop-up.
        assert_eq!(word(4) & 0x7f00_001f, 0x3500_0008);
    }

    #[test]
    fn guard_builds_the_pop_up_record() {
        let words = [word(0x20), word(0x24), word(0x28), word(0x2c)];
        assert_eq!(movz_movk(words), Some(POPUP_RECORD));
        assert_eq!(movz_movk([0, 0, 0, 0]), None);
    }

    #[test]
    fn hooked_sites_match_the_site_list() {
        assert_eq!(HOOKED_SITES, [POPUP_OFFSET]);
        assert_eq!(SITES.len(), 1);
    }
}
