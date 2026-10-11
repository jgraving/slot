# RG SP Bluetooth audio update v3.1

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

The main menu contains Pair headphones, Speaker and your saved headphones.
A checkmark marks the current output. A connects a saved headset or selects Speaker.
Y on a saved headset opens Connect / Forget. Forgetting requires confirmation.
Pair headphones opens a separate scan menu. Generic headers and idle status text are omitted.
Selections use the current theme's full-width highlight, existing font and native button hints.
B goes back one page. X saves a diagnostic report without adding another menu.
Pairing and connecting run in the background. Wait for completion before starting another operation.
Saved named devices appear on the main menu; MAC-only names are hidden from lists.
Forgetting requires confirmation. Use speaker disconnects and stops automatic reconnect.

A small Bluetooth symbol appears beside the carousel clock while the selected headphones
are connected. A read-only background check updates it every few seconds; it hides when
Speaker is selected or the headset disconnects.

## Send a failure report

When an operation fails, attach Bluetooth/latest-error.txt from your content SD card here.
It includes the pairing transcript, exact BlueZ error, adapter/service state, audio route,
installed emulator file list and recent slot.log. Older failures are retained in Bluetooth/errors.
Press X anywhere in the Bluetooth menus to save Bluetooth/latest-report.txt without needing a failure.
Automatic reconnect failures use Bluetooth/latest-restore.txt so unavailable headphones
at startup do not overwrite a manual failure report. Listing devices and saving reports
never wait for the reconnect lock. A manual scan, connect, Speaker or Forget action
asks the automatic reconnect to stop; it does not kill or restart Bluetooth services.
Reports contain Bluetooth names/addresses and application log paths; review before sharing publicly.

## Changes and validation

The pairing helper now maintains a registered NoInputNoOutput agent through discovery and pairing,
checks actual Paired/Connected properties, and waits for the A2DP Audio Sink service before routing.
Helper tests cover live-agent pairing, failure reporting, filtering, routing, restore and forgetting.
Rust checks cover list/state parsing, menu access, highlighted rendering, settings and audio behavior.
The v2 pairing/audio implementation was confirmed working by the RG SP user.
V3 simplifies the menus and adds the carousel indicator. V3.1 also fixes the startup race
seen in the supplied log: wait for the A2DP source before connecting, retry a transient
profile-unavailable response once, and give manual actions priority over automatic reconnect.
The ALSA backend and startup script are unchanged. The new UI/startup handling needs a device test.

## Restore

Power off and restore your backed-up System/slot and System/launch_frontend.sh.
Remove System/bluetooth.sh and Config/bluetooth-headphones, then reboot.

Source: BrandonKowalski/slot 9d7c795e7c862f37db912f5400f0ad004ebba1c8.
BaseOS reference: 1.3.0, 7dec5cba47db9b77b18b546a52acc51b1df64088.
