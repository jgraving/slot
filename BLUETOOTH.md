# RG SP Bluetooth audio update v2

MERGE this update into your existing slot installation. DO NOT replace or delete the System folder.
This update does not contain emulator cores. If the previous update replaced your entire System
folder, restore the original slot System folder first so games work again.

## Install

1. Power off and back up your existing System folder.
2. Copy the files inside this update's System folder into the card's existing System folder.
   Replace only matching files. Keep all existing emulator files, ROMs, saves and configuration.
3. Boot. Open Settings → Bluetooth Audio → Pair headphones.
4. Put headphones into pairing mode, select Scan for headphones, choose their name,
   then select Connect headphones. Sony WF-C510: earbuds in case, lid open,
   hold the rear case button at least five seconds until the blue light double flashes.

## Menus

Bluetooth Audio has separate Pair headphones, Saved headphones, Audio output and Diagnostics pages.
Selections use the current theme's full-width highlight and existing font. B goes back one page.
Pairing and connecting run in the background. Wait for completion before starting another operation.
Saved headphones shows paired named devices; MAC-only names are hidden from lists.
Forgetting requires confirmation. Use speaker disconnects and stops automatic reconnect.

## Send a failure report

When an operation fails, attach Bluetooth/latest-error.txt from your content SD card here.
It includes the pairing transcript, exact BlueZ error, adapter/service state, audio route,
installed emulator file list and recent slot.log. Older failures are retained in Bluetooth/errors.
Diagnostics → Save diagnostic report writes Bluetooth/latest-report.txt without needing a failure.
Reports contain Bluetooth names/addresses and application log paths; review before sharing publicly.

## Changes and validation

The pairing helper now maintains a registered NoInputNoOutput agent through discovery and pairing,
checks actual Paired/Connected properties, and waits for the A2DP Audio Sink service before routing.
Helper tests cover live-agent pairing, failure reporting, filtering, routing, restore and forgetting.
Rust checks cover list/state parsing, menu access, highlighted rendering, settings and audio behavior.
The ARM64 binary was compiled for the RG SP. Pairing/playback still require a hardware test.

## Restore

Power off and restore your backed-up System/slot and System/launch_frontend.sh.
Remove System/bluetooth.sh and Config/bluetooth-headphones, then reboot.

Source: BrandonKowalski/slot 9d7c795e7c862f37db912f5400f0ad004ebba1c8.
BaseOS reference: 1.3.0, 7dec5cba47db9b77b18b546a52acc51b1df64088.
