# OTA implementation stages

The `slot-update` crate verifies the frontend overlay produced by System builds.
It is an internal library, not yet an on-device menu or an installer.

A caller must supply the archive size and SHA-256 plus commit, channel and version
from trusted HTTPS release discovery. The package cannot choose those values for
itself. Stable and development channels are supported; PR preview downloads remain
manual. Never disable TLS verification to work around an unset device clock.
Checksums do not provide independent signing; GitHub/HTTPS is the trust boundary.

Verification rejects unknown schema/target/repository, identity mismatches,
unknown paths, emulator cores, traversal, symlinks and special files, duplicate
manifest entries, missing payloads, wrong file hashes/modes/sizes, excessive
expanded sizes, non-AArch64 frontends and inconsistent installed build identity.
The explicit allowed-path list must be reviewed when introducing a new System file.
Archive limit: 64 MiB; expanded total: 64 MiB; individual file: 32 MiB;
manifest: 64 KiB. Verification holds the bounded payload in memory.

Staging uses a unique temporary directory under an existing trusted persistent
parent, rejects symlink ancestors, writes only verified files, flushes payloads
and directories, then writes READY. An error removes the temporary directory.
A power cut may leave an incomplete directory; a future installer must ignore
it without READY and reverify staged hashes before installation. Staging never
changes the live System tree, executes downloaded scripts, restarts the device,
or touches games, saves, credentials or emulator cores.

Still to implement in separate PRs:

1. GitHub release discovery and bounded HTTPS download using BaseOS curl/CA roots.
   Bind all assets to one published release; keep stable and development separate.
2. Restart-time installation with an interruption journal, backup and rollback.
   Validate power-loss recovery in a filesystem test harness before hardware tests.
3. On-device channel selection, check/download/confirm/restart UI. Gate update
   controls through the optional button lock. Never install automatically.

Do not expose Install until recovery is implemented. Hardware tests are needed
for Wi-Fi, low-memory behavior and the complete restart/rollback path.
