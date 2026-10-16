# Sound outputs

Open **Sound**, then **Pair audio device**. Put your device in
pairing mode, choose **Scan for devices**, and select them from the list.

Saved devices appear in Sound. After connecting, slot returns to this list and
marks the selected output with a checkmark. A Bluetooth icon beside the battery
shows when the selected Bluetooth device is connected. The local output is shown
as **Wired headphones** when the headphone jack is in use, otherwise **Internal
speaker**. Press Y to manage a saved device, B to go back, or MENU to close the audio page. Volume uses slot’s
existing controls. Press X to save a diagnostic report in the SD card’s
`Bluetooth` folder.

The last selected Bluetooth device reconnects when slot starts and is retried
if it disconnects while slot is awake. Retry delays increase from 5 to 15, 30,
then 60 seconds. A passive BlueZ listener notices connection changes promptly;
if that tool is unavailable, polling remains as a fallback. The listener stops
while slot sleeps or Bluetooth is off. Reconnect never scans for or pairs a new device. Selecting the
local output stops automatic reconnect. **Bluetooth: Off** also powers off
and blocks the radio, and stays off after restarting. Pairings are kept. Select
a saved device or scan for devices to turn Bluetooth back on.

**Volume on disconnect** sets the local output volume when Bluetooth disconnects
or wired headphones are unplugged. It defaults to **0% (silent)** and changes in
5% steps. If Bluetooth disconnects while a cable is plugged in, this volume
applies to the wired headphones. Unplugging a cable while Bluetooth is playing
does not change the Bluetooth volume. Reconnecting the last Bluetooth device
restores the Bluetooth output’s volume and mute setting, including after a
restart. Bluetooth, speaker and wired output volumes are stored separately. A brief notice and the usual volume bar show each Bluetooth
connection change; unplugging wired headphones also shows a notice and volume
bar. Zero volume uses a muted icon.

Sound is also available from Settings in the in-game menu. Closing it returns
to the paused game menu. The battery indicator shows Bluetooth for wireless
audio or headphones for the active wired jack, on the carousel and in-game menu.

## Device support

Sound always includes the local output and disconnect-volume setting. Pairing and
saved Bluetooth devices appear only when Bluetooth support is detected. The
check runs in the background without starting the radio on current BaseOS.
If pairing is missing, the reason is saved in `Logs/bluetooth-support.log`.

On BaseOS, slot attaches the Bluetooth UART only when the adapter is absent,
then uses BlueZ to control power. Scanning and reconnecting do not rerun the
vendor low-level controller-enable commands. Startup waits up to about 20 seconds
for BlueZ to expose an initializing adapter (plus any in-flight command timeout).
Manual operations queue behind a running operation for up to 90 seconds; stale
locks are reclaimed and automatic reconnect yields to a manual choice. These
waits run off the UI thread. A persistent failure still reports its cause instead
of claiming success or retrying indefinitely. Audio uses BlueALSA and its ALSA plugin. Older BaseOS versions
without the hardware flag fall back to checking for a controller; other systems
need an adapter that is already available through BlueZ.

Incoming pairing and discoverability stay disabled. New pairing is initiated
only for a device selected in Sound. The temporary agent does not auto-approve
ambiguous authorization prompts or guess legacy PINs; devices requiring those
prompts are rejected with an error. A device is trusted only after its audio
service has been verified. Manual actions can interrupt automatic reconnect.
Connections are reported as successful only after the selected device’s A2DP
playback endpoint appears in BlueALSA. Failed connections leave the current
audio output unchanged. Automatic speaker fallback never cancels a reconnect
already in progress.

These controls reduce unnecessary exposure; they do not replace BaseOS security
updates or disable unrelated system Bluetooth services.

Bluetooth connects on RG SP and RG35XXSP. Audio is currently quieter on RG35XXSP
with the same device and game; the cause is still being investigated. These menu
and disconnect-setting changes need device testing.

## Development checks

- `python3 card/test-bluetooth.py`
- `cargo test -p slot --no-default-features --features device --lib --test quick_menu --test menu_repeat`
- `cargo test -p slot-ui --test quick_menu`

The helper tests simulate Bluetooth commands to check device detection, pairing,
connection failures, reconnect and competing operations without needing a radio.
They do not replace testing on the device.

For unexpectedly quiet audio, connect the device and press X in Sound, then check
`Bluetooth/latest-report.txt`. It includes slot’s volume settings and read-only
mixer readings. No gain boost or automatic change to device volume is applied.

Routine operation transcripts stay in temporary memory and are removed when the
operation ends. Saved diagnostics overwrite `latest-report.txt`, `latest-error.txt`
or `latest-restore.txt`; each helper report is limited to 128 KiB. The helper
removes only its older timestamped operation logs and error archives. Background
BlueALSA output is not appended to these files. Press X to save a fresh report
when investigating a problem.

## USB audio

Plug a USB audio device into the handheld’s data/OTG port. slot selects it
when its playback endpoint appears, including when starting with it plugged in.
A deliberate Sound operation already in progress takes priority. Selecting a
different output stays in effect until another USB attachment or restart.
Only USB devices with an ALSA playback endpoint are listed; internal audio,
HDMI and microphone-only devices are excluded.

USB playback is checked with digital silence before switching, without changing
hardware mixer gain. USB volume starts at 20% and its volume and mute setting are
remembered separately across restarts. The USB icon identifies the selected
output beside the battery and in the volume indicator, including in-game menus.
Unplugging USB applies Volume on disconnect before returning to the local output.
Turning Bluetooth off leaves USB audio playing. No microphone capture is opened.

AB13X USB Audio earbuds have played on RG SP, but a reported disconnect during
volume adjustment is still under investigation. The device disappeared from the
kernel's ALSA inventory. Press X in Sound after a failure to include recent USB
kernel messages, device power state, stream formats and mixer readings in the
bounded report. These are read only; slot does not change USB power policy or
hardware gain. Automatic selection and the updated icons need device testing. USB audio does not require Bluetooth
hardware. Card IDs are used rather than assuming the USB device is always card 3.

Bluetooth defaults to Off on a fresh setup. The Bluetooth row uses left/right to switch On or Off; selecting Pair audio device enables it automatically. Explicit On/Off is remembered, and existing saved-headphone setups retain startup reconnect. USB outputs use the same headphone symbols as the audio jack.

Bluetooth Off now uses BlueZ power control without rfkill-blocking the attached UART. On waits for the powered state with a bounded timeout. If the controller stays off, the menu asks for a full shutdown and power-on; this recovery hint is not shown for ordinary headphone connection failures. On-demand reports include Bluetooth kernel messages and rfkill state.
