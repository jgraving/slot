# Power options

Open **Power** from the carousel menu.

- **Lid Close:** Light Sleep (the default), Deep Sleep, or Power Off.
- **Shut Down After:** 1, 3 (the default), 5, 10, 15, or 30 minutes, or Never. This timer runs only while sleeping.
- **Sleep Now:** uses deep sleep when the lid setting is Deep Sleep; otherwise uses light sleep.
- **Power Off:** saves and plays the existing shutdown animation.

Holding the hardware power button retains its immediate shutdown behavior. Deep sleep uses the kernel's `mem` suspend mode when available. On the RG SP, wake deep sleep with Power; opening the lid does not wake it. The menu reports unavailable deep sleep instead of pretending it succeeded.

A timed deep sleep arms the RTC to wake and shut down after the chosen interval. A manual wake before that resumes the UI. If the alarm cannot be armed, suspend fails safely and the UI returns. Never does not arm an alarm. The previous future RTC alarm and sleep policy are restored afterward.

Save-state work is settled before suspend; the session waits for its emulator to pause, mutes audio during suspend, and clears stale controller gestures after waking. A failed suspend restores the screen and reports the failure.

Settings are saved in `Config/power.txt`. When it is absent, the earlier `lid_close` and `sleep_timeout` values are read from `Config/comfort.txt` without changing that file. Other old branch features are not imported.

Platform behavior is covered by mock tests. Real suspend, wake, Bluetooth recovery and RTC shutdown need device-specific testing. The lid watchdog follows these saved preferences, including Never; an older upstream `lid_off_min` preference is imported only when neither power nor comfort preferences exist.
