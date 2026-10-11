# Night display test build

Based on the fork’s main branch, including Bluetooth audio v3.3.

## Changes

Imports the backlight curve and tests from flo333’s upstream PR #54:
https://github.com/BrandonKowalski/slot/pull/54
Reviewed head: a371486f73a2748e9eb898bd1c2bcd89eea0d644.
For a 255-level backlight, steps are now 0, 1, 8, 23, 47, 77, 113, 155, 202, 255.
The endpoints remain off and full brightness. Middle steps are dimmer than before.

The existing warm-white/blue-light slider still has steps 0–9 and the same controls.
Maximum channel gains change from [1.0, 0.82, 0.62] to [1.0, 0.60, 0.20],
producing a stronger amber tint. Step 0 remains neutral. Existing saved slider values
remain valid, but nonzero settings now look warmer. Lower the slider if needed.

Warmth is a static RGB multiplier in the final rendering pass. This update does not
change display refresh timing, frame blending, or backlight PWM frequency. It does
reduce green and blue output, so colours and apparent brightness change. Actual
panel flicker at the newly available low backlight levels needs hardware testing.
The PR author reports testing level 1 without flickering on an RG SP.

## Install and test

Power off, back up System/slot, and merge this package’s System folder into your
existing v3.3 System folder. Replace matching files only; keep emulator executables.
Bluetooth pairing and the audio helper are unchanged.

Try brightness at a comfortable level first, then test the lower steps. Compare the
warmth slider at 0 and 9 in the menu and a game. If a very low brightness level flickers,
raise brightness and report the step and whether warmth was enabled.

To roll back, merge the v3.3 System update back over this installation. No save or
pairing configuration needs to be removed. This test branch has not been merged into main.
