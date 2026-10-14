# Wi-Fi setup

Wi-Fi is included in the current System build and does not require OTA.
Open the main options menu, choose **Wi-Fi** (below Sound), then **Find a network**.
Choose your WPA2 network, enter the password and press START. After connecting,
choose **Save encrypted credentials** to reconnect automatically on later boots.
**Set button lock** is optional and off by default.

Install the entire System overlay, replacing System/slot and adding System/network.sh;
keep existing emulator cores. If the Wi-Fi row is missing, check that you replaced
the executable on the active content card rather than creating System/System.
The late boot power-switch animation is removed in this version.

## Saved credentials

A random 256-bit key in BaseOS's `/data/slot-network/vault.key` encrypts the saved
profile with AES-256-GCM and a fresh random nonce on every write. Reopening does
not require a session password. The key directory is 0700 and key is 0600, with
owner, file type, length and symlink checks. The Wi-Fi passphrase is used to derive
the WPA2 key, then dropped from a zeroizing buffer; it is not persisted. A derived
WPA2 key is still a usable network credential and receives the same protection.

This protects copies of the encrypted profile without its key, including ordinary
content-card backups. It is NOT full-disk encryption or a hardware-backed secret:
BaseOS `/data` is on the OS SD card. Anyone with root access or the full OS card
can recover the key. Automatic reconnect without a user secret has that limit.
The optional four-button lock restricts UI operations, without protection
against offline attacks on removable storage.

Runtime integration verifies `/data` is persistent, prepares safe in-memory
configuration and disables secret-bearing crash dumps before loading credentials.
WPA2-Personal/AES is the initial scope; WPA3-only and enterprise networks need
separate implementations. Unknown vault versions and tampered data fail closed.

## Settings → Wi-Fi

The menu offers a WPA2/AES scan list, masked controller keyboard, session-only
connection, saved connection, Wi-Fi off and confirmed forgetting. D-pad moves,
A enters a character, X deletes, L1/R1 change character pages, START submits,
and B cancels. Open, WEP, WPA1, enterprise and WPA3-only networks are omitted.
One network is saved at a time. After connecting, explicitly choose Save to enable
automatic reconnect on later boots. Wi-Fi is off by default. Turn Wi-Fi off also
disables boot reconnect; Connect saved network enables it again without a password.

Both the encrypted profile and its key live in `/data/slot-network`. Runtime
preparation verifies that `/data` is ext4 and `/run` is tmpfs, locks the root
password and stops development SSH, disables swap/core dumps, and serializes radio
operations. The launcher also disables frontend core dumps. This deliberately
removes default-password remote login; it does not install a file-sharing server.
USB ADB, if enabled by BaseOS, is outside the network-lock boundary.

The active WPA configuration is sent on stdin, written to a 0600 RAM file and
removed after association or failure. No credentials are written to logs. DHCP
continues lease renewal in the background. Boot connection happens on a worker
thread and cannot delay the game carousel. Link play disconnects home Wi-Fi first;
reconnect manually after link play. Closing the Wi-Fi menu leaves the connection
active. System suspend/resume still requires hardware testing.

For device validation, test a correct/incorrect password, reboot reconnect,
Wi-Fi off across reboot, Forget, an absent router, suspend/resume, Bluetooth audio
and multiplayer on the RG SP. Automated fake-OS tests do not establish radio or
kernel compatibility. Copy updates by merging System files; retain emulator cores.

## Optional button-code lock

Set button lock is available in the Wi-Fi menu. It is off by default. Choose
four presses from A/B/X/Y or the D-pad; SVG button icons show each entry. START
submits, SELECT deletes and MENU cancels. Repeat the code to confirm. Once enabled,
opening the Wi-Fi controls requires the code; closing them or sleeping relocks.
The current unlocked menu offers Disable button lock. Five wrong attempts cause
a 30-second delay. An invalid saved lock fails closed.

The lock does not block games or saved automatic reconnect. It stores a salted
PBKDF2 verifier rather than the button sequence, but the small code space is
inherently weak against offline guessing. The delay is in-process, not a hardware
anti-tamper counter. A person with root or SD access can reset/remove it. Recovery
is to remove `/data/slot-network/button-lock` through trusted local OS access;
there is no online password recovery or secret master code. OTA installation and
scraper account changes must explicitly integrate this gate when implemented.
