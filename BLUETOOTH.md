# Bluetooth audio

Open **Bluetooth Audio**, then **Pair audio device**. Put your headphones in
pairing mode, choose **Scan for headphones**, and select them from the list.

Saved headphones appear on the main audio page. Press Y to connect or forget a
saved device, B to go back, or MENU to close the audio page. Volume uses slot’s
existing controls. Press X to save a diagnostic report in the SD card’s
`Bluetooth` folder.

The last selected headphones reconnect when slot starts. Choosing **Internal
speaker** stops automatic reconnect. If your headphones disconnect, audio stays
silent until they reconnect or you choose the speaker.

## Device support

Bluetooth Audio appears when BaseOS reports Bluetooth hardware and the required
audio software is installed. Devices without support keep their existing menu.
The check runs in the background without starting the radio on current BaseOS.
If the option is missing, the reason is saved in `Logs/bluetooth-support.log`.

On BaseOS, slot uses the existing Bluetooth startup script and service when you
scan or connect. Audio uses BlueALSA and its ALSA plugin. Older BaseOS versions
without the hardware flag fall back to checking for a controller; other systems
need an adapter that is already available through BlueZ.

Pairing is enabled only during Bluetooth operations and disabled afterwards.
The adapter is not made discoverable. Manual actions can interrupt an automatic
reconnect. Failed connections leave the current audio output unchanged.

The earlier version was tested on RG SP. This version still needs testing on the
device, including pairing, restart, volume, disconnects and lid sleep/wake.
Other models have not been tested.

## Development checks

- `python3 card/test-bluetooth.py`
- `cargo test -p slot --no-default-features --features device --lib --test quick_menu --test menu_repeat`
- `cargo test -p slot-ui --test quick_menu`

The helper tests simulate Bluetooth commands to check device detection, pairing,
connection failures, reconnect and competing operations without needing a radio.
They do not replace testing on the device.
