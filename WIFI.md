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
