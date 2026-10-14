# Downloadable builds

Every pull request gets a checked ARM64 `slot-System-preview-…` Actions artifact.
Open the PR's **System builds** run and download the artifact. Unzip that download,
then unzip `slot-System.zip`. With the device powered off, merge its `System`
folder into SD2's existing `System` folder. Keep emulator cores and other existing
files. Back up your existing System folder first. No ROM, save or label changes
are needed. Preview artifacts expire after 7 days and require GitHub sign-in.

After this workflow is merged, each push to main publishes a separate Development
prerelease at https://github.com/jgraving/slot/releases. Each keeps its own tag,
source commit and assets; no moving tag or overwritten package. Development means
merged main, not a bundle of unreviewed PRs.

Push a reviewed `vMAJOR.MINOR.PATCH` tag to build and publish a stable release.
Both channels run checks before packaging or publishing. Stable releases also
include the existing full fresh-card ZIP with emulator cores and their licenses.
A manual workflow run builds a preview only. PRs cannot publish releases and
receive no release write permissions. Nothing publishes until this is merged.

Each release has the same `slot-System.zip`, `manifest.json` and `SHA256SUMS`
asset names. The manifest identifies schema 1, h700-aarch64, channel, version,
repository, full commit and every installed file's size, SHA-256 and mode.
`System/build.json` records the installed identity. Packages contain only the
tracked System overlay plus the built frontend; never runtime credentials or
user content. They do not update BaseOS or replace existing emulator cores.

Updates are installed manually from the SD card. Slot has no internet updater.
