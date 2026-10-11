# Cartridge sounds

Built on main after Favorites/Recents and Grab and Go were merged.

Rolling past a cartridge plays the original quiet plastic click again,
with five subtle pitch variants. Idle shelves and shelf changes do not click. Fast
scrolling is limited to one tick every 35 ms, without queuing a backlog.

Sound → Cartridge sounds controls the effect. It defaults to On and survives reboot.
The preference carries over from Interface sounds. Existing cartridge insertion and
ejection sounds are unchanged. Bluetooth connection/disconnection sounds are removed.

All sounds respect output volume and mute. With Mute on disconnect enabled, the
speaker still switches to volume zero. The notice now reads “Device disconnected · Volume 0%”
in the standard shaded top notification strip, using the same size and fade timing
as Day/Night mode. It does not interrupt gameplay.

Merge the supplied System files into the existing System folder, replacing matching
files only. Retain emulator executables. No save/load or Cart Studio format changes.
Restore the previous System/slot to roll back.

Preview/cartridge.wav is for auditioning only; the effect is built into the executable.

## Power and menus

Settings are ordered Screen, Sound, Gameplay, Power, Date & Time, About.
Power → Lid Close offers Light Sleep (default), Deep Sleep, and Power Off.
Power → Sleep Timer shuts down after 3 Minutes (default), 15 Minutes, or 30 Minutes.
Never disables automatic shutdown. The timer applies to both kinds of sleep.

Holding Power retains Restart and Power Off, with Sleep added below. Sleep uses
deep sleep when Lid Close is set to Deep Sleep; otherwise it uses light sleep.
Power Off shows a short-travel SP-style switch with a broad grey grip, three raised
ribs, and moulded OFF/ON markings. The click is four times the cartridge click's
amplitude and remains controlled by output volume. Restart switches off, pauses,
then switches on again, with a click at each detent. Sleep from the power menu
closes the screen into a fading line over 320 ms before turning off the backlight
and suspending. Closing the physical lid still sleeps immediately.
Automatic shutdown while sleeping stays silent.

Switch visual reference: https://gbasp.ru/japangreen.html (Pearl Green side view).

Deep sleep uses BaseOS Super Standby. **Press Power to wake; opening the lid alone
does not wake it.** Timed sleep arms the RTC alarm, wakes at the deadline, and shuts
down. If the alarm cannot be armed, deep sleep reports unavailable instead of
silently sleeping indefinitely. Existing save/resume handling is retained.

BaseOS reference: https://github.com/pvaibhav/BaseOS/blob/main/docs/05-runtime-power-network.md

Device testing is still needed: check Power wake, speaker/headphone audio after
wake, and a full three-minute deep-sleep shutdown on your handheld. Software tests
simulate the sleep deadline; they cannot verify the device's RTC wake support.
