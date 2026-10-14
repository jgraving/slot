# Review sequence for connected features

1. **Saved Wi-Fi storage:** authenticated encryption with a local random key,
   no required session password. Explicit removable-card security limits.
2. **Wi-Fi connection UI/runtime:** use BaseOS tools, masked controller input,
   opt-in boot reconnect, protected RAM config and default-password SSH disabled.
3. **Optional button-code lock:** off by default, four controller presses rendered
   as SVG button graphics. Gate connected-feature settings; allow ordinary games
   and saved Wi-Fi reconnect. Rate-limit wrong entries. This is a convenience/access
   lock, not protection against an attacker reading or modifying the OS SD card.
4. **OTA updates:** release discovery and an explicit Install action. Verify TLS
   with BaseOS CA roots; never bypass verification for an unset clock. Stage an
   allowlisted frontend package separately, verify size/digest and release identity,
   retain the old executable, install on restart and recover from interrupted or
   unsuccessful updates. Preserve ROMs, saves, settings and emulator cores. Test the
   installer and recovery separately before exposing installation in the menu.
   Release packaging/authenticity and rollback need a dedicated PR and hardware gate.
5. **Personal metadata and art:** use each user's own account through a supported
   provider integration, with secrets in private storage and never command arguments
   or logs. A normal ScreenScraper user login does not replace an application's
   developer credentials. Do not copy credentials out of an existing scraper.
   Resolve supported provider access and BaseOS-compatible tooling before promising
   a direct ScreenScraper integration. Fetch only the user's games, cache results,
   respect provider quotas, and preview before saving optional sidecars.
6. **On-device game-details/editor UI:** consume optional versioned JSON/PNG files,
   with clean fallbacks for missing/bad files. Keep upstream cartridge-label and ROM
   formats unchanged. This replaces the need for a separate Studio workflow for
   supported features without embedding a desktop browser on the handheld.

Each substantial unit gets a separate PR. Do not merge the sequence as a single
unreviewed batch. Establish actual RG SP network behavior before shipping OTA or
live scraping; shell mocks and host Rust tests cannot establish driver behavior.
