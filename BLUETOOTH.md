# Sound outputs

Open **Sound** from the carousel settings or the in-game Settings menu to choose
an audio output and manage headphones. The selected output has a checkmark.
Speaker, headphone and Bluetooth icons identify the output in the menu and
volume display. USB and analog headphones use the same headphone symbol.

## Bluetooth headphones

Bluetooth defaults to **Off** on a fresh setup. Change the **Bluetooth** setting
with left/right, or choose **Pair audio device** to enable it automatically.
Put the headphones in pairing mode, choose **Scan for devices**, and select
them from the results. Progress indicators show background work.

Saved devices appear in Sound. Press **A** to connect. Press **Y** to forget a
device, then **Y** again to confirm; **B** or navigation cancels the confirmation.
Press **B** to go back or **MENU** to close Sound.

The selected paired device reconnects at startup and is retried after connection
loss while slot is awake. Retry delays increase from 5 to 15, 30, then 60 seconds.
Automatic reconnect does not scan for or pair new devices. Selecting the local
output stops automatic reconnect.

The Bluetooth On/Off choice is remembered. Off powers down the controller through
BlueZ without rfkill-blocking the UART and preserves saved pairings. If the
controller cannot start, Sound asks the user to turn the handheld off and on.

## USB and wired headphones

The local output is shown as **Wired headphones** when the headphone jack is in
use, otherwise **Internal speaker**.

Plug a supported USB audio device into the handheld's data/OTG port. slot selects
it when its playback endpoint appears, including at startup. A manual Sound
operation already in progress takes priority. A deliberate output choice remains
in effect until another USB attachment or restart.

USB discovery uses ALSA card IDs rather than fixed card numbers. Only USB playback
endpoints are listed; microphone-only, internal and HDMI endpoints are excluded.
Playback is checked with digital silence before selecting the route.

## Volume

Volume and mute are remembered separately for the speaker, analog headphones,
Bluetooth and USB. USB starts at 20%. Use slot's normal volume controls.

**Volume on disconnect** chooses the local output volume after Bluetooth loss or
wired/USB headphone removal. It defaults to **0% (silent)** and changes in 5% steps.
If the headphone jack is occupied when Bluetooth disconnects, the setting applies
to the wired headphones.

Device and mute icons make the active output clear. Starting a game muted or at
zero volume briefly shows the volume bar, including when a game resumes at startup.

### Bluetooth volume synchronization

Linked Bluetooth volume is experimental and **not yet reliable in hardware
testing**. The handheld and headphones may still have separate volume levels.
If Bluetooth playback is quiet at maximum handheld volume, adjust the headphones'
own volume as well. This resolved quiet playback on the tested Sony WF-C510.

The implementation attempts native AVRCP volume through BlueALSA when the selected
A2DP transport exposes it. It checks readback before bypassing software attenuation
and falls back to software volume if native control is unavailable or fails.
Initialization accounts for the existing volume instead of setting the headphones
to maximum. Volume work runs outside the UI and audio threads.

The intended behavior is for handheld controls to adjust headphone volume and
headphone controls to update slot's indicator. Automated checks exercise this path,
but successful playback does not establish that two-way synchronization works.

## Tested devices and known limitations

Tested on **RG SP** and **RG35XXSP**, with Bluetooth playback and the Sound menu
broadly working on both.

- USB-C headphone playback has been confirmed on RG SP.
- USB audio was unavailable in RG35XXSP testing. A hardware limitation is
  suspected, but the cause has not been confirmed.
- Quiet analog headphone output was reported with some headphones, while the
  speaker and working USB-C headphones sounded normal. The exposed mixer levels
  were already at maximum; a general fix has not been established.
- A USB-to-3.5 mm adapter was not recognized. Previously failing USB earbuds also
  stopped working on a phone, so those dropouts are suspected hardware trouble.
- Abrupt Bluetooth power-off may take time to be reported by the Bluetooth stack.

Bluetooth controls appear only when support is detected. Local and detected USB
outputs remain available without Bluetooth hardware.

## Diagnostics

Press **X while in Sound** to save a report. **There is no on-screen hint for this
shortcut.** Reports are saved on the SD card:

- `Bluetooth/latest-report.txt`: the current manual report.
- `Bluetooth/previous-report.txt`: the previous manual report, for comparison.

Reports include selected output, saved volumes, Bluetooth state, mixer readings
and relevant USB driver, power and stream information. For quiet audio, save a
report with the affected output selected. For connection failures, save one after
the failure. If Bluetooth controls are missing, also check
`Logs/bluetooth-support.log`.

Routine operation transcripts remain temporary. Saved helper reports are bounded
to 128 KiB; error and reconnect reports use `latest-error.txt` and
`latest-restore.txt`. The helper cleans up only its own obsolete logs.

## Implementation and development checks

Audio uses BaseOS's existing BlueZ, BlueALSA and ALSA support. Adapter readiness
and the selected A2DP playback endpoint are checked before reporting a successful
connection. Competing operations are serialized, and automatic work yields to
manual choices. A passive BlueZ listener monitors connections while awake, with
polling as a fallback.

Incoming pairing and discoverability remain disabled. Pairing is initiated for a
device selected in Sound; ambiguous authorization prompts and legacy PIN requests
are rejected. Devices are trusted after their audio service has been verified.

Run the focused checks with:

- `python3 card/test-bluetooth.py`
- `cargo test -p slot --no-default-features --features device --lib --test quick_menu --test menu_repeat`
- `cargo test -p slot-ui --test quick_menu`

The checks cover device discovery, pairing, connection failures, reconnect,
competing operations, volume handling and menu behavior. Hardware validation
should include playback, both Bluetooth volume controls, mute, reconnect and
switching outputs. Automated checks do not replace device testing.
