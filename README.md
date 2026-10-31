# ssbu-quiet-elite-switch

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

A Skyline plugin for Super Smash Bros. Ultimate 13.0.5. On the Quickplay
character select, switching from a fighter in Elite Smash to one that is not
brings up a pop-up. This plugin stops that pop-up; the screen still switches
back from the Elite Smash colours.

**Before** (no plugin):

![Before: the pop-up appears](docs/media/before.gif)

**After** (with the plugin):

![After: no pop-up](docs/media/after.gif)

*Eden, 2026-10-30: Sephiroth is in Elite Smash, so the character select
opens as Quickplay (Elite Smash). Selecting him, then Pyra/Mythra (not in
Elite Smash): without the plugin the game stops on "You haven't unlocked
Elite Smash with this fighter. Returning to standard matchmaking."; with it,
the screen goes straight back to Quickplay and Ready to Fight.*

- One hook, guarded: it installs only on 13.0.5 with the hooked code
  unchanged ([details](docs/HOOK.md)).
- **No clashes:** if another plugin wants the same instruction, the two
  share one hook through `ssbu_inline_site_1a1caa8_v1` (found with
  `nn::ro::LookupSymbol`, any load order) and both keep working. A plugin
  that hooks it without sharing is chained onto instead.
- **Small:** 40 KiB of game memory once loaded (code and data), no heap use.
- UI only: no rating, network or match state changes.

**Tested:** Eden, 2026-10-30, 13.0.5 on LAN Online with a rating runtime:
0.1.0 install line `result=installed role=owner hooks=popup
heap_allocations=0`, pop-up gone as above. 0.2.0 with a stand-in for a
plugin that hooks the same instruction without sharing (diagnostic build,
`foreign=inline`, then `foreign=far`), next to gsp-visualizer and
elite-rematch: `popup=branch/near` and `popup=chain/far`, `result=installed`,
pop-up gone, and the stand-in ran too. Not tested on Nintendo's servers or a real Switch.

## Install

Copy `libssbu_quiet_elite_switch.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. The
install line goes to `sd:/ultimate/ssbu-quiet-elite-switch/events.log`.

## Build

```text
cargo test
cargo skyline build --release
```

## License

MIT
