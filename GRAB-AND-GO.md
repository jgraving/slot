# Grab-and-go refinements

Built on the tested Favorites and Recents branch, with Bluetooth and night-display changes.

## Browsing memory

Each shelf remembers its selected cartridge. After reboot with no game seated, the
carousel opens the last browsed nonempty shelf. With no browsing history it retains
the previous Recents-first behavior. A seated game still resumes, and a one-game card
still starts automatically. Missing cartridges safely fall back to an available one.
Positions are saved after a short idle interval and before launch, sleep or shutdown.

## Day and night presets

SELECT + X switches presets on the carousel or in a game. Screen → Night Mode offers
the same toggle, including A to activate it. A brief toast shows the selected mode.

Day initially uses your current brightness and warmth. Night initially uses brightness
2 and warmth 9. Adjust brightness and warmth with the existing SELECT + direction
shortcuts while either mode is active; those values become that mode's remembered
preset. Both presets and the active mode survive reboot. No schedule is imposed.

## Headphone volume memory

Each Bluetooth address gets its own volume and mute preference. Use the existing
volume buttons; there is no new volume menu. New headsets start at the existing
headphone volume. Speaker and wired-headphone settings remain separate. A newly
paired device with a different Bluetooth address is treated as a different headset.

## Quiet disconnect (optional)

Sound → Mute on disconnect defaults to Off. When enabled, a confirmed headset
disconnect sets internal speaker volume to zero and switches back to that output.
Gameplay continues. An overlay over the current screen says “Headphones disconnected”
and “Speaker volume 0”. Raise the volume when you want sound again.

The output remains muted while the switch completes, even if the headset reconnects.
If the switch fails, it stays silent. Headset volume memories are retained. The existing
quiet preference is reused, so users who enabled Pause on disconnect now get this
behavior. Save/load behavior is unchanged.

Sound lists Internal speaker and saved devices first, followed by Pair device and
Mute on disconnect. Device rows are visually grouped. Status messages occupy a small
fixed area above the menu; initial loading is silent and scanning does not shift rows.

With this option enabled, unavailable Bluetooth outputs also remain silent at startup.
Detection is asynchronous, normally about a second; command timeouts retain the last
confirmed connection state.

## Compatibility and installation

Preferences live in Config/comfort.txt, separate from Cart Studio files, cartridge
artwork, Favorites/Recents lists and game saves. Save/load controls and save-state
formats are unchanged. The Bluetooth pairing helper is unchanged.

Power off, back up System/slot, then merge the supplied System files into the existing
System folder. Replace matching files only; retain all emulator executables. Do not
replace the whole folder. No reflashing or cartridge rebuilding is needed.

To roll back, restore the previous System/slot. Earlier versions ignore comfort.txt.
The current brightness and warmth remain ordinary slot settings.
