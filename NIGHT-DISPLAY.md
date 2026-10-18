# Night display

Hold Select and press X to switch between saved day and night brightness/warmth settings. X alone keeps its usual behavior. The shortcut works on the carousel and in games.

The shortcut can be remapped under **Controls → Shortcuts → Day / Night**. Reset All restores Select + X.

Night starts at brightness 2 and warmth 9; day starts from your existing display settings. Adjust brightness and warmth with the existing shortcuts while either preset is active to remember its values. The active preset survives restart. A brief toast confirms the change.

Preferences are stored in Config/display.txt. If it does not exist, the night/day/bedtime settings from the old Config/comfort.txt are imported without modifying that file or its other preferences. Invalid brightness or warmth entries are ignored.
