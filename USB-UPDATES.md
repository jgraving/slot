# Update slot over USB on a Mac

A BaseOS build with USB mass-storage maintenance mode can expose the games card
to a computer over the handheld's USB-C port. The SD card stays in the handheld.
Wi-Fi, Samba and a separate transfer application are not required.

## Enter storage mode

1. Power the handheld off completely.
2. Connect it to your Mac using a USB **data** cable. A charging-only cable will
   charge the device but cannot transfer files.
3. Hold **MENU** while powering on. Keep holding MENU until BaseOS displays
   **USB STORAGE: EJECT BEFORE RESTART**. If cable connection starts the device
   automatically, hold MENU before connecting; use Power if it remains in charging mode.
4. Find the games volume under **Locations** in Finder. With a second card fitted,
   BaseOS exports TF2; otherwise it exposes TF1's user-data partition.

The games volume needs a filesystem macOS supports, such as FAT32 or exFAT.
USB storage exposes the existing filesystem; it does not convert it. An ext4 card
will not become a Finder-readable drive just because it is connected through USB.
Slot remains stopped throughout this maintenance boot so the handheld and Mac do
not write the same filesystem at the same time.

## Copy the update

1. Download and extract the device update from
   [this fork's releases](https://github.com/jgraving/slot/releases), or the tested
   build attached to the relevant change. Use a build for your device.
2. Back up the installed `System/slot` executable to your Mac. If the update includes
   helper scripts, back up the installed copies of those scripts as well.
3. Open the update's `System` folder and the games volume's existing `System` folder.
   Copy the supplied files into the existing folder, replacing matching files only.
   For a frontend-only update, this usually means `slot` and any supplied helpers.
4. Retain the existing emulator files, including `mgba_libretro.so` and
   `gpsp_libretro.so`. A partial update may not contain them. Merge any supplied
   subfolders rather than replacing the installed folders wholesale.
5. Leave `Games`, `BIOS`, `Labels`, `Config`, `Saves` and `States` on the card intact.
   Do not copy empty top-level folders from a release over these folders.
6. Eject the games volume in Finder and wait for it to disappear. Disconnect the
   cable and restart the handheld **without MENU held**.

On macOS, replacing an entire folder can remove installed files absent from the
update. Copying the supplied files into the existing `System` folder avoids that
problem. To roll back, repeat storage mode and restore your backed-up files.

## If the volume does not appear

- In Finder settings, enable external disks in the sidebar; also check Disk Utility
  with **View → Show All Devices**. If the disk appears there but not in Finder,
  inspect its partition/filesystem and whether the volume is mounted.
- Try a known data cable, a direct Mac port, and a complete power-off before entering
  storage mode. Connect the cable before normal startup; a later plug-in can select
  the wrong USB role on this hardware.
- If slot starts normally, maintenance mode was not selected. Retry MENU from power-on
  and check that the installed BaseOS build includes this feature.
- If macOS says the disk is unreadable, cancel the prompt. Do not initialize or erase
  a card containing your games and saves. Check the filesystem/build before writing.

## Validation status

This documents an existing BaseOS feature; it does not change its USB driver or
storage-export implementation. BaseOS's published hardware validation covers an
RG40XX V, including TF1 export to macOS and eject/restart. Its documentation still
lists whole-card TF2 enumeration/write/eject testing as pending. RG SP with TF2 on
macOS must therefore be tested before treating this workflow as confirmed on that
combination.

For an RG SP test:

- [ ] MENU maintenance boot displays the storage message and slot stays stopped.
- [ ] The expected TF2 games volume appears in Finder with its existing contents.
- [ ] Create a small temporary text file, eject, and confirm the file after reboot.
- [ ] Replace the frontend with a backed-up, known-compatible build; eject and confirm
      startup, games, saves, custom labels and emulator files remain usable.
- [ ] Repeat storage mode and confirm restoring the backed-up frontend works.

References:

- [BaseOS USB storage and macOS validation](https://github.com/pvaibhav/BaseOS/blob/7dec5cba47db9b77b18b546a52acc51b1df64088/docs/08-usb-adb-and-otg.md)
- [BaseOS user instructions](https://github.com/pvaibhav/BaseOS/blob/7dec5cba47db9b77b18b546a52acc51b1df64088/README.md#usb-access)
- [macOS filesystem formats](https://support.apple.com/guide/disk-utility/dsku19ed921c/mac)
