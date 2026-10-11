# Cartridge clicks

Built on main after Favorites/Recents and Grab and Go were merged.

Rolling past a cartridge plays the same quiet plastic tick as the first sound build,
with five subtle pitch variants. Idle shelves and shelf changes do not click. Fast
scrolling is limited to one tick every 35 ms, without queuing a backlog.

Sound → Cartridge clicks controls the effect. It defaults to On and survives reboot.
The preference carries over from Interface sounds. Existing cartridge insertion and
ejection sounds are unchanged. Bluetooth connection/disconnection sounds are removed.

All sounds respect output volume and mute. With Mute on disconnect enabled, the
speaker still switches to volume zero. The notice now reads “Disconnected · vol 0”
in the standard shaded top notification strip, using the same size and fade timing
as Day/Night mode. It does not interrupt gameplay.

Merge the supplied System files into the existing System folder, replacing matching
files only. Retain emulator executables. No save/load or Cart Studio format changes.
Restore the previous System/slot to roll back.

Preview/cartridge.wav is for auditioning only; the effect is built into the executable.
