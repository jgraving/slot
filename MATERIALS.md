# RG SP painted shell and matte background

## Photo-derived paint grain

The grain comes from the Pearl Blue RG SP photographed by Nick for Retro
Handhelds, not generated noise or the retired pebbled slot bitmap.

- Article: https://retrohandhelds.gg/anbernic-rg-sp-first-impressions/
- Photograph: https://rh-handhelds-content.nyc3.cdn.digitaloceanspaces.com/2026/07/PXL_20260730_030922058.jpg
- Source dimensions: 1914 × 1080 pixels.
- Source SHA-256: `110796e96976d5bd9c38352c9502b8590e92046d04ac475db0f217fc176fd8c3`.
- Clean deck patch, clockwise from top-left: (820,800), (955,781),
  (980,916), (837,941). This excludes buttons, markings and speaker holes.

`tools/extract-shell-material.py` rectifies that quadrilateral to 128 × 128,
converts it to luminance, rejects isolated dust highlights, subtracts Gaussian
lighting, and solves a periodic-plus-smooth Poisson decomposition to remove
seams. Grain is normalized to a restrained 2.1 luminance-level RMS and clipped
to ±8. Only this unrecognizable surface signal is embedded; no device photograph,
logo or lettering is shipped. This is photo-derived image texture, not a scan
of physical roughness, normals or a calibrated PBR material.

The extraction script produces only the paint grain. Original SVG geometry supplies
the slot silhouette, lip, molded ribs and recessed screws. Both layers are tinted
once and cached for each shell. The carousel and power switch use the same paint
sampler and RGB palette. The switch retains its neutral recess, slider and timing.

The four hand-tuned palette approximations are Graphite, Platinum, Pearl Blue
and Pearl Pink, based on the official finishes and the review photography:
https://anbernic.com/products/rg-sp. They are not colorimeter measurements.

## Settings

General → Shell Color cycles Match Theme, Graphite, Platinum, Pearl Blue,
Pearl Pink. It updates the carousel housing immediately and the shutdown switch
uses that same saved selection. Match Theme preserves the prior behavior:
dark uses Graphite and light uses Platinum. Explicit colors are independent of
the light/dark text theme. Cards without `shell_colour` keep Match Theme.

## Wallpaper

`tools/build-tray-wallpaper.py` now creates a plain muted petrol matte surface
with a soft lighting falloff and very fine grain. The rejected tray outlines,
cartridge impression, grip marks and colored studs have all been removed.
Custom wallpapers are unchanged.

## Reproduction and preview

With Python, Pillow, NumPy and SciPy installed:

```sh
python tools/extract-shell-material.py /path/to/PXL_20260730_030922058.jpg
python tools/build-tray-wallpaper.py
cargo run -p slot-ui --example shell_finish_preview -- /tmp/shell-finishes
python tools/compose-shell-preview.py /tmp/shell-finishes previews/rgsp-shell-finishes.png
```

The preview exports the same textures and draw commands used in production.
The GB/GBC bodies match the narrower inserting body of the GBA, whose top grip remains wider.
This is a screen layout choice, not physical screen sizing.

## GBA SP insertion housing, SVG rebuild

`crates/slot-ui/assets/slot_back.svg` and `slot_front.svg` are original vector
geometry, replacing the old photographic masks and extracted fasteners entirely.
Proportion references: iFixit, [GBA SP bottom shell replacement](https://www.ifixit.com/Guide/Nintendo+Game+Boy+Advance+SP+Bottom+Shell+Replacement/12735),
photos `2kN4kwkSbK6PyaUD`, `OCO3qBtqtET3xTk4`, and `Ldow6GCmi5DecKU3`.
The supplied close-up sets the final proportions: flat shoulders ease into a
curved central scoop with a flat middle; the two ribs are open-bottom inverted U
shapes. A larger recessed metal screw carries an explicit Y-shaped tri-point
pocket. The rounded outer SP shoulders and both recessed corner case screws are restored.
No reference photograph is embedded in these SVGs. Red encodes paint relief;
neutral elements preserve the dark throat and small metal screw. The original
fine RG SP paint grain is applied after SVG rasterization for every finish.
Sparse subpixel vector scratches follow the handled edge, with paired lighter
and darker strokes suggesting shallow wear rather than a coarse leather bump.
Cartridge aspect ratios and insertion timing are retained.

## Motion validation

Insertion and shelf departure use one width-derived scale for both axes, preserving
the source cartridge aspect ratio exactly instead of interpolating rounded endpoint
widths/heights independently. All three cartridge types meet the same connector stop; taller GB/GBC paks
retain their extra height outside the shell. Tests sample GBA, GB and GBC over the full
stroke at 720 and 640 pixels, including the actual enlarged carousel start and
rear/cart/front draw ordering.

The earlier ad-hoc GIF used scale 1.0 rather than the enlarged carousel scale;
the checked-in exporter now always obtains both position and scale from Shelf.
Reproduce the reviewed insertion/ejection preview with:

```sh
cargo run -p slot-ui --example shell_finish_preview -- /tmp/slot-motion animation gba
python tools/compose-slot-motion.py /tmp/slot-motion /tmp/slot-motion.gif
```

Use `gb` or `gbc` in place of `gba` for the other cartridge shapes.

The full front and oblique reference photographs establish the slot volume:
the curved cut edge has a narrow inner bevel. The lower shell is flat, without
a raised front face, lower bulge or cast shadow. The rear layer includes the
metal liner behind the inserted cart. The outer SP shoulders and corner screw wells bound a centered 476px housing,
fully visible on both 640px and 720px canvases. Its lower body continues below
the screen, like a close view of the real SP. The two small inverted-U ribs remain.

## Seating calibration and marking legibility

- GBA caliper measurements: 60.0 mm across the cap, 56.9 mm across the body,
  34.5 mm high; [original measurement photographs](https://nantoka-game.com/?p=318).
- GB/Color family body: approximately 57 × 65.5 mm; [iFixit cartridge reference](https://www.ifixit.com/Device/Nintendo_Game_Cartridge).
- Inserted GB reference: [Willem Middelkoop's own SP repair photograph](https://willem.com/en/2017-03-26_repairing-a-nintendo-gameboy-advance-sp/).
- The two supplied inserted GBA photographs calibrate the visible grip.

The existing art includes transparent horizontal margins, particularly on GB/GBC;
its opaque body, rather than the entire texture rectangle, sets physical width.
Preserve the uniform source aspect ratio. The GBA cap is 308px wide and its 340/360 inserting body is ~291px;
the GB/GBC texture is 315px, whose 288/312 opaque body is also ~291px. The same widths are used
on the carousel and when seated, avoiding any size jump or horizontal squash. A shared connector stop ~28 mm beyond the straight mouth plane
leaves roughly 7 mm of GBA grip and 37 mm of GB/GBC body outside that plane.
The central scoop reveals a little more. This depth is a photographic estimate,
not an official engineering measurement or a claim of physical screen sizing.
The old common-top alignment incorrectly swallowed tall paks and is removed.

The switch's ON/OFF paths and both solid molded pips now have stronger upper-left
highlights and lower-right shadows. The same paint palette and grain remain;
timing, slider geometry and sound are unchanged.

The enlarged GB/GBC shelf placement is centered in the usable area below the HUD,
rather than underneath its top plate. The 340px-tall GB/GBC drawing leaves about 14px of clearance below the HUD
and above the housing. The cartridge
geometry checks retain fractional uniform scaling instead of requiring the
scaled lower edge to land on an integer pixel.

The red-circled reference identifies the aperture's recessed end returns. Narrow
rounded inner walls and contact shadows at x=233..246 and x=474..487 restore this
volume inside the slot. The independent outer device shoulders and corner screw wells are also restored. The end walls belong to the rear layer, so the cartridge grip sits in front of
them. Only the curved front cut edge occludes the cartridge body.

The lower lip's former face gradient and drop/contact shadow are removed. Depth
comes from the recessed end walls and slot interior; below the cut edge the
shell uses the same uninterrupted fine paint as the rest of the surface.

The visible interior liner is matte silver metal, not a black cavity or painted
plastic. A neutral cool-silver lighting gradient and sparse subpixel vertical
brushing describe the metal. Only the narrow contact below the upper plastic lip
stays dark. Liner pixels remain untinted across all four shell finishes; this is
an appearance reference, not a claim about the original metal alloy.

The moving switch slider uses uniform smooth plastic fills. Broad cap/grip
reflection gradients and the bright lower gloss streak are removed. Only thin
molded edges, the thumb groove and a local contact shade remain; the extracted
paint grain is never applied to this slider. Housing paint and markings retain
the selected finish.

## Larger SP housing and matching insertion bodies

The vector art is now 720×72 instead of 720×58. The aperture is 308px wide, matching the visible GBA grip, with its original curved-to-flat cut edge,
matte metal liner and recessed end walls preserved. Rounded shoulders and two
corner screw wells follow the supplied full SP photograph. The actual device
outline is centered at x=122..598 and remains fully visible on both panels.

The 308px GBA top grip remains wider than its ~291px inserting body. GB/GBC
bodies match that narrower body, not the top grip. Their 315px texture includes
transparent margins and is proportionally 340px tall, comfortably between
the 40px status area and housing at y=408. Every source retains its native
aspect ratio. GB/GBC transparent margins are counted in the texture size but
not the visible width. Insertion uses the same sizes and common connector stop.
The tiny catch creep scales with approach distance so a tall pak already close
to the mouth still visibly pauses. Production draw tests check every frame of
all three types at 640×480 and 720×480 for aspect ratio, inserting-body width and upper clipping.

The central screw assembly counteracts the aperture transform: the recess, metal
head and Y-shaped driver pocket all receive equal horizontal and vertical scale.
A raster regression measures the untinted metal head and rejects oval bounds.

## Persistent top status

Battery/percentage, charging and audio-route indicators are at the same 16px top
corners as the in-game overlay, with the clock at the top right. The carousel
and settings use their theme ink; the game overlay retains its white HUD ink.
Status stays visible in settings subpages, Bluetooth/audio, Date & Time and About.
The physical SP housing at the bottom no longer carries status text or icons.
Draw-command tests exercise the carousel and each settings surface and verify
that clock, percentage and audio icon appear once inside the 40px top band.

The latest shoulder reference replaces the separate upright end-wall piece
with one softly rolled plastic corner. Its surface turns in plan and section
into the opening, using a continuous curved profile and broad, low-contrast
shading. The outer surface stays in front; the inner return remains behind
the cartridge grip. The independent center screw stays circular.

The screw recess, rim and metal head use the same center. Three equal driver
arms meet at that center, with a raster centroid check for the dark pocket.

The shoulder material fades continuously into the flat deck before the slot
boundary. No rectangular clip or vertical seam divides those two surfaces.

## Slot boundary and fit refinement

The 308px mouth is bounded at x=206..514, exactly matching the GBA grip,
removing the former 9px gap at each end. The GB/GBC face's rendered opaque
mid-body spans 288/312 of its texture (the prior 292/312 estimate included
transparent edge pixels). A 315px texture therefore gives a ~291px body,
matching the 340/360 GBA insertion body. Uniform scaling produces a ~340px
tall GB/GBC pak, still below the HUD. Raster coverage checks measure both
visible widths and the real liner boundary instead of assuming texture bounds.

The inner shoulder profile now eases tangentially into the flat lip. Both layers
use the same broad shoulder shading, fading horizontally and vertically into
the deck. The front relief and bevel are clipped to its actual silhouette;
the rear relief excludes the metal and the rib strokes stop at the rear edge.
The center screw is at y=16.125, halfway between the top edge (1px) and liner
(31.25px), with its circular dimensions preserved. Both corner assemblies use
one shared local drawing: shallow recessed seats, the same viewing direction,
and a centered head and driver cut. These are visual approximations.
