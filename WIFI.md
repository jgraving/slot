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

## Connecting on the handheld

Settings → Network → Find a network lists WPA2-Personal networks using AES/CCMP.
Select a network and enter its password. The on-screen keyboard uses the D-pad to
move, A to type, X to delete, L1/R1 to change character pages, START to submit and B
to cancel. Passwords and vault passphrases remain masked. Hidden networks,
enterprise authentication and WPA3-only networks are not supported in this version.

On the first successful connection, choose Connect for this session (nothing
saved) or Save encrypted credentials and create/confirm an unlock passphrase. Unlock saved network decrypts the saved profile and reconnects after
restart. Disconnect keeps the vault unlocked in memory for this session; Disconnect
and lock drops keys and disconnects. Forget requires confirmation and deletes the
vault. Saving a different network replaces the single saved profile.

Before accepting credentials, the launcher disables core dumps and persistent
swap, stops BaseOS's default password SSH daemon and checks these operations.
Networking fails closed if this preparation fails. The supplicant configuration is
written only under `/run/slot-wifi` and unlinked after association. DHCP renewal
runs in the background. Passwords never appear in subprocess arguments or logs.

Device validation: check a correct/incorrect password, special characters, DHCP
renewal, reboot/unlock, lock/forget, airplane-mode routers, audio after sleep and
multiplayer. Link play takes over the radio; use Reconnect saved network afterward.
No claim of hardware validation is made by the automated tests.
