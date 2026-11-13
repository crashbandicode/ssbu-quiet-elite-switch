# The pop-up and the hook

2026-10-30. Static analysis of the 13.0.5 `main`. Addresses are offsets from
the start of its `.text`, the base the plugin adds them to. Rechecked
2026-11-13 against the decompressed `.text` of the 13.0.5 NSO (sha256
`eb79e639…`): the 64 guarded bytes, the calls and strings below, and that
the only `ldrb`/`strb` at `#0xf8e` are the two named here.

## What the game does

`1a1c9f0(css, _, is_elite)` switches the Quickplay character select between
its Elite Smash and Quickplay looks when the selected fighter's Elite Smash
status changes. It does nothing unless the CSS can show Elite Smash
(`css + 0xf8d`), and keeps the current look in `css + 0x16a`.

- **To an Elite fighter:** plays `black_base_vip` and `title_off`, sets
  `+0x16a`. No pop-up.
- **To a fighter not in Elite Smash:** plays `black_base_def` and `title_on`.
  Then, if `css + 0xf8e` is clear, it looks up pop-up record
  `0x58ffff206396e7a9` (`3291150`), parses it into the shared pop-up
  (`32edf70`, the parser elite-rematch also uses) and shows it (`32f1bb0`),
  and sets `+0xf8e`. So the pop-up appears once per visit to the character
  select.

`+0xf8e` is read only at `1a1caa8` (`ldrb w8, [x20, #0xf8e]`, followed by
`cbnz w8` past the pop-up) and written only at `1a1cb0c`.

## What the plugin does

One inline hook at `1a1caa8`: it sets `css + 0xf8e` (x20 + 0xf8e) before the
game reads it. The game then takes its own "already shown" branch: no
pop-up, the look still switches, the flags are set as usual. Nothing else
changes; the hook writes only that byte of the CSS object.

## Guards and other plugins

- **Sharing the hook (inline-hook sharing v1, `src/share.rs`, the same file
  as gsp-visualizer's).** Two inline hooks on one instruction collide. So a
  plugin that wants `1a1caa8` exports `ssbu_inline_site_1a1caa8_v1()`,
  returning a table with `register(callback)`, and looks the name up with
  `nn::ro::LookupSymbol` before hooking; every plugin gets the same answer
  in any load order. The plugin whose table comes back installs the one hook
  and calls every registered callback from it; the others register and
  install nothing. Both keep working. The install line says
  `role=owner … guests=<n>` or `role=guest owner=<address>`.
- **A plugin that hooks it without sharing (0.2.0, `src/chain.rs`, the same
  file as gsp-visualizer's, the rules elite-rematch uses).** The 64 bytes at
  `1a1caa8` (the flag read and the four instructions that build the pop-up
  record) must be the 13.0.5 ones apart from other plugins' Skyline hooks.
  Another plugin's far jump on the site gets chained: this hook goes on a
  small stub that then jumps on to that plugin's handler, so both run, this
  one first. Another plugin's one-word branch is relocated into this hook,
  which is kept only if it is a one-word branch too (otherwise the bytes are
  put back). Hooks further into the 64 bytes are accepted. The install line
  says `popup=<stock|chain|branch>/<near|far>`; anything else there gives
  `popup=foreign/skipped result=not_installed` in
  `sd:/ultimate/ssbu-quiet-elite-switch/events.log`. A hook that does not
  check out is put back (`/restored`), and a far jump to a handler that is
  not mapped executable is left alone (`chain/skipped`); both also give
  `result=not_installed`. Another game version gives
  `result=unsupported_version`, and an owner that turns this plugin down
  `result=owner_refused`.
- No branch lands inside the 16 bytes a hook may rewrite at that address.
- Plugin API v1, for other plugins through `nn::ro::LookupSymbol` (any load
  order): `ssbu_quiet_elite_switch_hooked_sites_v1(out: *mut usize,
  capacity: usize) -> usize` returns the offsets it hooked (`0x1a1caa8`;
  none unless it installed the hook itself, as the owner), so another plugin
  can leave it alone.
