# Favorites and Recents

Based on this fork's main branch, including Bluetooth audio and the warmer night display.
Inspired by https://github.com/BrandonKowalski/slot/issues/57.

- L1/R1 cycles Recents → Favorites → GBA → GB → GBC, skipping empty shelves.
- SELECT toggles the selected cartridge's favorite status on any shelf. A short
  confirmation appears; a small star marks favorites over their existing labels.
- Favorites are alphabetical and can mix all three systems. Up/Down still jumps letters.
- Recents contains up to 25 successfully loaded games, newest first, without duplicates.
  Up/Down letter jumping is disabled on this shelf.
- Boot opens Recents when available, unless a cartridge was seated. The existing
  single-cartridge automatic boot behavior is preserved.

## Cart Studio compatibility

Games, PNG labels, shell choices, core settings and save paths keep their existing
formats and locations. No cartridge faces need rebuilding. Cart Studio exports can
be copied onto the card as before; merge folders and retain existing files.

Membership lives only in Config/favourites and Config/recent, one platform/stem per
line, for example `gba/Metroid Fusion`. These are device-local lists, not a new
cartridge format. The platform is part of each key, so identically named GB and GBA
games remain distinct. Missing ROMs are hidden without deleting their membership;
restoring the same filename brings them back (subject to Recents' 25-entry limit).
Renaming a ROM gives it a new identity. Cart Studio does not edit these lists.

## Install

Power off and back up System/slot. Merge the supplied System files into your existing
System folder, replacing matching files only. Keep emulator executables and all other
files. Do not replace the entire folder. There is no need to reflash BaseOS or rebuild
cartridges. The archive contains no Config, Games, Labels or Saves folders.

To roll back, restore the previous System/slot executable. The two membership files
can remain; older slot versions ignore them. Test this branch on the handheld before
merging it into main.
