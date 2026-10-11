# Tactile interface sounds

Built on main after Favorites/Recents and Grab and Go were merged.

- Rolling past a cartridge plays a quiet plastic tick, with five subtle pitch variants.
  Idle shelves and shelf changes do not click. Fast scrolling is limited to one tick
  every 35 ms, without queuing a backlog.
- Bluetooth connection plays a short plug seating sound; disconnection plays a softer
  release sound. Cues follow confirmed transitions, not repeated polling results, and
  wait half a second for the output route to settle.
- Sound → Interface sounds controls these three new effects. It defaults to On and
  survives reboot. Existing cartridge insertion/ejection sounds are unchanged.
- All sounds use the normal output volume and mute setting. Mute on disconnect keeps
  the disconnection cue silent; the existing “Speaker volume 0” overlay still appears.
- Sounds are synthesized locally from damped resonances and filtered noise; no external
  recordings, services or downloaded audio assets are used.

Merge the supplied System files into the existing System folder, replacing matching
files only. Retain emulator executables. No save/load or Cart Studio format changes.
Restore the previous System/slot to roll back. Previous builds ignore the new sounds
preference in Config/comfort.txt.

The archive's Preview folder contains WAV versions of the three sounds for auditioning.
They are not installation files; the effects are built into the executable.
