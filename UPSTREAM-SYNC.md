# Upstream integration — 2026-10-18

Merged BrandonKowalski/slot through `1b20432089eb84a340466c7ac61a6cf39d589576` (nine commits after the previous shared base), into jgraving/slot only.

## Included

- Global console, shortcut, and shelf control remapping with confirmed per-page Reset All.
- Left-stick D-pad input on devices that expose a supported stick; RG34XXSP support remains unverified upstream.
- Asynchronous core/ROM opening so loading does not block cartridge animation.
- Wi-Fi driver unload on power off, and a separate lid watchdog.
- SD-card repair with confirmation and a bundled static exFAT checker/source/license.
- mGBA flash-bank state-load fix. Upstream removed run-ahead because it could corrupt 128 KB GBA flash saves; this fork follows that removal.
- Multi-device deployment support and upstream changelog/site updates.

## Fork-specific resolutions

- Retained shell SVGs, all four finishes, fine grain, large carousel/proportional insertion, gold favorites and removal confirmation, custom link port, power animation/click, top-corner status indicators, Bluetooth/USB routing and volumes, and persistent day/night presets.
- Day / Night is now a remappable shortcut, defaulting to Select + X. Repeated key-down events do not toggle it repeatedly.
- The existing Power page remains authoritative. It gains 1/5/10-minute choices alongside 3/15/30 minutes and Never. Existing saved indices retain their meanings. An upstream `lid_off_min` is imported only when no fork power/comfort file exists.
- The lid watchdog reads the same power/comfort preferences and honors Never, rather than overriding it with an independent three-minute timeout.
- Every long settings page, including Controls and the expanded main menu, retains a six-row viewport and scrollbar. Binding hints keep their native aspect ratio and 32px height. Control pages retain the fork's shell instead of displaying upstream's older plugged-slot bitmap.
- Shortened two deep-sleep messages so they fit at full toast text size; the previously failing full-size toast unit check now passes.
- Fork README, public release downloads and release automation remain. Integration branches build/test without publishing; only main/version tags publish releases.

## Validation

Local device-feature compilation and focused regression checks cover the combined behavior. The release workflow runs those checks again on native ARM64, builds the emulator cores and static exFAT checker, checks binaries/source materials, and packages both System updates and SD2 starter downloads.

macOS compositor screenshot tests cannot be executed on the Linux host. Physical SD repair, controller/accessory compatibility, suspend/wake and final visual behavior still require hardware testing.
