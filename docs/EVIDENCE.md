# Evidence

2026-11-13. What the README's "Tested" line rests on, gathered from the run
folders and card copies named here. Nothing below is newer than its date.

## Eden

The Yoga, Eden-GIF AppImage `7b4d1862…`, 13.0.5, LAN Online with the rating
runtime, Sephiroth seeded into Elite Smash (`EDEN_LAN_FIGHTER_SEEDS=82:1700`).

| Run | Start (UTC) | Plugins | The plugin's log |
| --- | --- | --- | --- |
| e13 | 2026-10-30 23:04 | none | none; `before.gif` (the pop-up appears) |
| e14 | 2026-10-30 23:08 | 0.1.0 release `8e08dbd4…` | `result=installed role=owner hooks=popup guests=0 heap_allocations=0`; `after.gif` (no pop-up) |
| e15 | 2026-10-30 23:53 | 0.2.0 diag `a53711ad…` with `foreign=inline`, gsp-visualizer `d660002a…`, elite-rematch `689c423f…`, a share probe | `foreign=inline role=owner popup=branch/near guests=0 result=installed heap_allocations=0`, `diag suppressed total=1`, `diag standin popup_inline=1 popup_far=0` |
| e16 | 2026-10-31 00:30 | the same, `foreign=far` | `foreign=far role=owner popup=chain/far guests=0 result=installed heap_allocations=0`, `diag suppressed total=1`, `diag standin popup_inline=0 popup_far=1` |

`foreign=` makes the diagnostic build hook the site first as a plugin that
does not share would ([HOOK](HOOK.md)); `standin` counts that hook's runs.

Files, on the Yoga under `/home/intpa/gsp-visualizer-work/`: `solo-eNN.log`
and `PREP-eNN.json` (plugins and sha256), `runs/eNN/console.log` (no
`panic`, `abort`, `crash` or `Unmapped` in any of the four), and the
plugin's log in `profiles/y/user/sdmc/ultimate/ssbu-quiet-elite-switch.before-eMM/events.log`,
which holds the run before `eMM` (e14 in `.before-e15`, e15 in `.before-e16`,
e16 in `.before-e17`).

## Switch

- 2026-10-30: 0.1.0 (`8e08dbd4…`) added over DBI MTP, then replaced by the
  0.2.0 release build (`c65d9cc2…`); both receipts end `installed_verified`
  (`/home/intpa/gsp-visualizer-install/switch-mtp-quiet010/` and
  `switch-mtp-quiet020/deployment.json`).
- 2026-11-02: the card held 16 plugins, this one included
  (`/home/intpa/ssbu-cpu-pilot-install/card-2026-11-02/plugins.sha256`). Its
  `events.log` (`card-2026-11-02c/`) had four boots of
  `plugin=ssbu-quiet-elite-switch/0.2.0 role=owner popup=stock/near guests=0 result=installed heap_allocations=0`.
  The shipped build logs only that line, so no log shows the pop-up's
  absence on the console.
- 2026-11-12: still `c65d9cc2…` (readback in
  `/home/intpa/smash-training-lobby/artifacts/card-install-detector-0.5.1/verify.json`).

## Size

`tools/build/nro_size.py` (ssbu-mod-playbook) on `c65d9cc2…`: text 20,480,
rodata 12,288, data 4,096, bss 4,096; resident 40,960 bytes. 0.1.0
(`8e08dbd4…`, `crate-type = ["cdylib", "rlib"]` at `2917c9a`) was 475,136.
