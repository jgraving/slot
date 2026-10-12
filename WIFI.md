# Wi-Fi and wireless updates

## Saved credentials

Slot uses BaseOS's radio, WPA supplicant and DHCP tools. No separate application or
service needs managing. Enter the Wi-Fi password once; saved networks reconnect
automatically after restart.

`Config/wifi.vault` contains the SSID and derived WPA2 key encrypted using
AES-256-GCM with a fresh random nonce for every save. The Wi-Fi password is never
saved. The random 256-bit encryption key lives in BaseOS's private
`/data/slot-network/vault.key`, with root-only directory/file permissions (0700/0600).
It is not stored with the games-card files, derived from a public serial/MAC address,
or included in update exports. There is no extra unlock password.

This prevents casual plaintext exposure of the games-card profile. It does not
protect against an attacker who controls the OS or reads both the profile and the
private system key from a stolen device. This is the convenience/security tradeoff
for unattended reconnect on this hardware. The derived WPA2 key is itself a usable
network credential, so it must remain protected even though the password is absent.

Keys are needed in RAM while connected. Before entry/connection, slot disables
crash dumps and persistent swap, stops stock password SSH and locks root password
authentication. The supplicant configuration is RAM-only and unlinked after
association. Secrets never appear in command arguments or logs. Secret buffers are
zeroed on drop; this cannot guarantee removal of every OS copy.

The first version supports one saved WPA2-Personal network using AES/CCMP. Hidden
networks, WPA3-only networks and enterprise authentication are not yet supported.
Existing games, cartridge labels, settings and save formats are unchanged.
