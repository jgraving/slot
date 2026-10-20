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
- Day / Night is now a remappable shortcut, defaulting to Select + X. Repeated key-down events do not toggle it repeatedly. Deep-sleep wake invalidates the shortcut-table cache after clearing held inputs, so remapped shortcuts continue to work.
- The existing Power page remains authoritative. It gains 1/5/10-minute choices alongside 3/15/30 minutes and Never. Existing saved indices retain their meanings. An upstream `lid_off_min` is imported only when no fork power/comfort file exists.
- The lid watchdog reads the same power/comfort preferences and honors Never, rather than overriding it with an independent three-minute timeout.
- Every long settings page, including Controls and the expanded main menu, retains a six-row viewport and scrollbar. Binding hints keep their native aspect ratio and 32px height. Control pages retain the fork's shell instead of displaying upstream's older plugged-slot bitmap.
- Shortened two deep-sleep messages so they fit at full toast text size; the previously failing full-size toast unit check now passes.
- Fork README, public release downloads and release automation remain. Integration branches build/test without publishing; only main/version tags publish releases.

## Validation

Local device-feature compilation and focused regression checks cover the combined behavior. The release workflow runs those checks again on native ARM64, builds the emulator cores and static exFAT checker, checks binaries/source materials, and packages both System updates and SD2 starter downloads.

macOS compositor screenshot tests cannot be executed on the Linux host. Physical SD repair, controller/accessory compatibility, suspend/wake and final visual behavior still require hardware testing.


## 2026-10-21: controls, native layouts and high refresh

Integrated BrandonKowalski/slot through `f4b7bc3b1f7240c00d245c82fe123181a2d5c590`: unified control routing with contexts, chord clash warnings and remappable Turbo A/B; panel-sized wallpaper/bezel layouts; link legends and silent-test defaults. The fork retains its shell artwork, cartridge geometry, audio controls, saved night presets and sleep preferences. Night Toggle participates in the new global shortcut contexts, and deep sleep resets the new binding-entry cache.

Selectively adapted the panel driver and native game pacing from [pvaibhav/slot commit e8ff757](https://github.com/pvaibhav/slot/commit/e8ff757e3712ea021463b5449ae2fcf1deebe157), rather than merging unrelated features. Credit to Prashant Vaibhav. Only the exact known RG34XX SP/RG34XX model, driver and 720×480 timing combinations are eligible. Register writes are checked, failed changes attempt rollback, and wake restores the display before backlight. Other panels remain unchanged.

This fork additionally targets **120 FPS for carousel/menu animations** when the platform reports 120 Hz enabled, otherwise 60 FPS. Emulated games use their core's native frame rate and audio sample rate; a faster panel does not double game speed. The existing time-based animations retain their durations. `high_refresh=1` is the default in `Config/slot.state`; set it to `0` with Slot stopped to opt out. An absent key migrates to enabled; unsupported or failed panels still use the 60 FPS UI clock. The preference is retained on transient failure, while the actual hardware mode determines pacing.

Validation includes panel identification, register rollback and wake ordering tests, simulated 60/120 Hz scanout at native game cadence, UI deadline counts, preference persistence and failed/unsupported fallback. ARM64 CI builds and tests the resulting package. Physical panel smoothness and sustained 120 FPS on a handheld cannot be established on this host and still need device review.
