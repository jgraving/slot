# Bluetooth audio

Open **Sound**, then **Pair audio device**. Put your device in
pairing mode, choose **Scan for devices**, and select them from the list.

Saved devices appear in Sound. After connecting, slot returns to this list and
marks the selected output with a checkmark. A Bluetooth icon beside the battery
shows when the selected Bluetooth device is connected. The local output is shown
as **Wired headphones** when the headphone jack is in use, otherwise **Internal
speaker**. Press Y to manage a saved device, B to go back, or MENU to close the audio page. Volume uses slot’s
existing controls. Press X to save a diagnostic report in the SD card’s
`Bluetooth` folder.

The last selected Bluetooth device reconnects when slot starts. Selecting the
local output stops automatic reconnect.

**Volume on disconnect** sets the local output volume when Bluetooth disconnects
or wired headphones are unplugged. It defaults to **0% (silent)** and changes in
5% steps. If Bluetooth disconnects while a cable is plugged in, this volume
applies to the wired headphones. Unplugging a cable while Bluetooth is playing
does not change the Bluetooth volume. Reconnecting the last Bluetooth device
restores its volume and mute setting from before the disconnect during the
current session. A brief notice and the usual volume bar show each Bluetooth
connection change; unplugging wired headphones also shows a notice and volume
bar. Zero volume uses a muted icon.

## Device support

Sound always includes the local output and disconnect-volume setting. Pairing and
saved Bluetooth devices appear only when Bluetooth support is detected. The
check runs in the background without starting the radio on current BaseOS.
If pairing is missing, the reason is saved in `Logs/bluetooth-support.log`.

On BaseOS, slot uses the existing Bluetooth startup script and service when you
scan or connect. Audio uses BlueALSA and its ALSA plugin. Older BaseOS versions
without the hardware flag fall back to checking for a controller; other systems
need an adapter that is already available through BlueZ.

Pairing is enabled only during Bluetooth operations and disabled afterwards.
The adapter is not made discoverable. Manual actions can interrupt an automatic
reconnect. Failed connections leave the current audio output unchanged.

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
