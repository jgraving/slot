# Bluetooth audio

This change adds headphone pairing, reconnect, output selection and forgetting to
upstream SLOT. Open **Bluetooth Audio**, then **Pair audio device**. Put the
headphones in pairing mode and select **Scan for headphones**. Saved headphones
appear on the main audio page; Y opens their management menu. B returns and MENU
closes the audio page. X saves a diagnostic report under `Bluetooth/` on SD2.

The last selected headphones reconnect on startup. Selecting **Internal speaker**
stops reconnecting them. If selected headphones disconnect, audio stays silent
until they reconnect or you explicitly select the speaker. Volume uses SLOT's
existing controls. No Wi-Fi settings, OTA, file sharing or additional network
services are introduced.

## Capability detection

The menu is hidden until a background check finds `bluetoothctl`, `bluealsa`,
`timeout`, a BlueALSA PCM advertised by `aplay -L`, and a real BlueZ controller.
Missing dependencies, initialization failures and probe timeouts leave it hidden.
The probe does not scan, enable pairing, or start BlueALSA.

On BaseOS, `/mnt/vendor/ctrl/setBluetooth.sh` initializes the radio and the existing
Bluetooth service is started. The Realtek attachment recovery is confined to this
backend. On other systems, an already initialized BlueZ controller is required;
no vendor initialization or service-manager assumptions are made. Pairing starts
BlueALSA's A2DP source only if needed and checks that the source is registered
before connecting. Operations are serialized; a manual output selection can
cancel an automatic reconnect. Pairability and discoverability are disabled after
operations.

The UI and audio routing do not depend on a device model name. This is not a claim
that every device is supported: its kernel, firmware and OS must provide the
required stack. The earlier implementation was tested on RG SP; this extraction
onto current upstream still needs an on-device regression test. Other hardware
has not been verified.

## Checks

- `python3 card/test-bluetooth.py` mocks the OS stack and tests detection, pairing,
  connection failures, A2DP validation, restore, cancellation and locking.
- `cargo test -p slot --no-default-features --features device --lib --test quick_menu --test menu_repeat`
- `cargo test -p slot-ui --test quick_menu`

For device review, check pairing, restart/reconnect, speaker selection, headphones
turning off, volume, lid sleep/wake and returning from the audio menu. On a device
without Bluetooth, check that the original settings list is unchanged.
