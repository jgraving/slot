# RG SP painted shell and quiet tray

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

The extraction script also retains the existing carousel's silhouette/alpha
while recovering the original insertion-lip cross-section and both small U-shaped
fasteners as a separate structural-relief layer, without the old coarse grain or
play emblem. The thumb scoop retains its original alpha contour and a rolled lip.
The resulting neutral geometry and shared grain are tinted once and cached for
each shell. The carousel and power switch use the same paint sampler and RGB
palette. The switch retains its neutral recess, slider and existing timing.

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

`tools/build-tray-wallpaper.py` builds an original quiet molded storage tray:
a satin neutral surface, shallow cartridge impression, a single perimeter lip,
three embossed grip ribs and three small colored studs. Both default and blurred
wallpapers replace the wood/sticker artwork. Custom wallpapers are unchanged.

## Reproduction and preview

With Python, Pillow, NumPy and SciPy installed:

```sh
python tools/extract-shell-material.py /path/to/PXL_20260730_030922058.jpg
python tools/build-tray-wallpaper.py
cargo run -p slot-ui --example shell_finish_preview -- /tmp/shell-finishes
python tools/compose-shell-preview.py /tmp/shell-finishes previews/rgsp-shell-finishes.png
```

The preview exports the same textures and draw commands used in production.
Cartridge sizes remain the accepted GBA +15%, GB/GBC +8%; no physical-sizing
experiment is included.

## GBA SP insertion housing, SVG rebuild

`crates/slot-ui/assets/slot_back.svg` and `slot_front.svg` are original vector
geometry, replacing the old photographic masks and extracted fasteners entirely.
Proportion references: iFixit, [GBA SP bottom shell replacement](https://www.ifixit.com/Guide/Nintendo+Game+Boy+Advance+SP+Bottom+Shell+Replacement/12735),
photos `2kN4kwkSbK6PyaUD`, `OCO3qBtqtET3xTk4`, and `Ldow6GCmi5DecKU3`.
These show a shallow bowed lip, two molded ribs, and a central case screw.
No reference photograph is embedded in these SVGs. Red encodes paint relief;
neutral elements preserve the dark throat and small metal screw. The original
fine RG SP paint grain is applied after SVG rasterization for every finish.
Sparse subpixel vector scratches follow the handled edge, with paired lighter
and darker strokes suggesting shallow wear rather than a coarse leather bump.
The accepted cartridge sizes and insertion timing are retained.
