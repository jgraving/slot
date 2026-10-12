# Wi-Fi and wireless updates

## Credential protection

Saved Wi-Fi credentials are encrypted in `Config/wifi.vault`. AES-256-GCM
protects confidentiality and detects modification; PBKDF2-HMAC-SHA256 with a random
16-byte salt and 600,000 iterations derives the encryption key from a separate
unlock passphrase (at least 12 characters). Every save uses a fresh random nonce.
Neither the Wi-Fi password nor the unlock passphrase is saved. The encrypted vault
contains the network name and its derived WPA2 key, which is itself a credential.

Use a long, unique unlock passphrase. A short or predictable passphrase can still
be guessed offline. There is no device-ID key, shared secret, or adjacent key file.
Unlock is required after reboot. Forgetting the unlock passphrase means deleting
the vault and entering the Wi-Fi credentials again; there is no recovery backdoor.

The WPA2 key is necessarily available to the supplicant in memory while connected.
Its configuration belongs in `/run`, never on either SD card. Secret buffers are
zeroed on drop; this reduces retention but cannot protect an unlocked device from
root access, malicious software, or every copy made by the OS. Crash dumping must
be disabled before credentials are entered. Card permissions alone are not treated
as protection, because removable FAT/exFAT cards do not enforce Unix permissions.

The vault uses one saved WPA2-Personal network in this first version. Wi-Fi and
transfer features are optional; existing cartridge, label, save and settings
formats are unchanged.
