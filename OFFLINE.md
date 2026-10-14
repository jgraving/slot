# Connectivity

Slot has no home Wi-Fi settings, saved network credentials, scraper, PIN, or
internet updater. Install downloadable System builds using the SD card.
Bluetooth headphone audio remains available.

Local wireless link play remains available. Opening its menu does not enable
Wi-Fi; selecting Host or Join does. Ending, cancelling, failing, or leaving a
session turns the interfaces down and blocks Wi-Fi. A background guard also
cleans up a session if the frontend exits. Link play uses the existing local
network and does not configure a gateway or connect to a saved home network.
It retains upstream's shared link password; it is not private authenticated
pairing and should only be enabled when playing with a trusted nearby device.

On frontend startup, the offline policy blocks Wi-Fi, disables IPv6 and IP
forwarding, stops remote-login/file-sharing and internet background services,
and masks BaseOS development-service executables for this boot. It keeps
Bluetooth services running. If masking fails, link setup is refused. A small
runtime guard catches late BaseOS radio initialization and stops services that
restart. The policy does not modify the immutable BaseOS image; it cannot
protect the interval before Slot starts, or another OS booted from the card.

The first launch deletes Slot's old `/data/slot-network` credentials and key,
and its RAM supplicant configuration. The retired `System/network.sh` is shipped
as a disabled stub so an overlay install replaces old copies. Deleting files
is not forensic secure erasure on flash; old backups and card images may still
contain credentials. No games, saves, labels, or Bluetooth pairings are removed.

BaseOS USB ADB is also disabled using `/data/no-adb` and stopped for this boot.
No SSH/SFTP, Samba, or FTP service is provided by Slot. Installing a different
frontend later does not automatically remove `/data/no-adb`.
