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
Power Off shows the photo-referenced Flame Red GBA SP switch with a recessed shell
surround, charcoal slider, raised grip, and moulded OFF/ON markings. The click is four times the cartridge click's
amplitude and remains controlled by output volume. Restart switches off, pauses,
then switches on again, with a click at each detent. Sleep from the power menu
closes the screen into a fading line over 320 ms before turning off the backlight
and suspending. Closing the physical lid still sleeps immediately.
Automatic shutdown while sleeping stays silent.

Switch visual reference: https://documents.cdn.ifixit.com/pdf/ifixit/guide_12889_en.pdf
(cover photo). The original vector reconstruction is split into `power_switch.svg`
and `power_switch_slider.svg` in `crates/slot-ui/assets`. Both are embedded and
rasterised once at startup; the animation moves the slider texture without parsing
SVGs or uploading frames during shutdown. The grip mark aligns with the OFF and ON
dots at the two detents. The switch keeps the approved 5:2 aspect ratio, centred at
600 by 240 pixels in the frontend's 720 by 480 canvas.

Deep sleep uses BaseOS Super Standby. **Press Power to wake; opening the lid alone
does not wake it.** Timed sleep arms the RTC alarm, wakes at the deadline, and shuts
down. If the alarm cannot be armed, deep sleep reports unavailable instead of
silently sleeping indefinitely. Existing save/resume handling is retained.

BaseOS reference: https://github.com/pvaibhav/BaseOS/blob/main/docs/05-runtime-power-network.md

Device testing is still needed: check Power wake, speaker/headphone audio after
wake, and a full three-minute deep-sleep shutdown on your handheld. Software tests
simulate the sleep deadline; they cannot verify the device's RTC wake support.

Options → Power → Shell Color cycles with Left/Right through Flame Red (the
original/default artwork), Platinum, Graphite, Pearl Blue, and Pearl Pink. The
four RG SP finish names and visual colour references come from Anbernic's current
RG SP product page, https://anbernic.com/products/rg-sp (checked 2026-10-13), not the
older RG35XXSP or RG34XXSP. These are photo-matched approximations, not manufacturer
paint specifications. The preference is saved as `shell_colour` in
`Config/comfort.txt`; missing or unknown values retain Flame Red. Changing finish
reuses the existing textures and recolours only the surround; the slider remains
charcoal. Power Off and Restart use the same selected finish.
