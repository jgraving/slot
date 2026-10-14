# Saved Wi-Fi: storage foundation

This first PR adds credential storage only. The connection menu and optional
button lock are separate changes; installing this commit does not enable Wi-Fi.

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
A later four-button lock will restrict UI operations, not claim protection
against offline attacks on removable storage.

Runtime integration must verify `/data` is persistent, prepare safe in-memory
configuration and disable secret-bearing crash dumps before loading credentials.
WPA2-Personal/AES is the initial scope; WPA3-only and enterprise networks need
separate implementations. Unknown vault versions and tampered data fail closed.

## Settings → Wi-Fi

The second PR adds a WPA2/AES scan list, masked controller keyboard, session-only
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

Before merging the runtime PR, test a correct/incorrect password, reboot reconnect,
Wi-Fi off across reboot, Forget, an absent router, suspend/resume, Bluetooth audio
and multiplayer on the RG SP. Automated fake-OS tests do not establish radio or
kernel compatibility. Copy updates by merging System files; retain emulator cores.
