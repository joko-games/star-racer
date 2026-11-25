# The installer and the updates

Since v1.0.0-rc6, at the user's word on 2026-11-24 ("un installeur moderne pour windows", "un auto
updater"), the Windows game comes as an installer and keeps itself up to date. Both are
[Velopack](https://velopack.io)'s, the successor of Squirrel, which the Release workflow runs.

## What the player sees

`StarRacer-win-Setup.exe`, the latest release's (the site's DOWNLOAD links to it), installs the game
in one click and a few seconds: no wizard, no administrator, for the user alone (in
`%LocalAppData%\StarRacer`), with Star Racer in the Start menu and on the desktop under the game's
logo ([The logos](logo.md)), the logo shown while it installs, and an entry in the system's
installed apps to remove it. The game starts once it is installed.

Each time the installed game starts, it looks at the latest GitHub release, quietly. A newer version
is downloaded in the background while the player plays, then:

- the first screen shows **VERSION X IS READY** with **RESTART**, which closes the game, puts the
  new version in place and starts it again;
- otherwise the next start puts it in place by itself, before the window opens.

A game not installed (`run.ps1`, `run-release.ps1`, a build of the workspace, the Linux build)
looks for nothing: Velopack says it is not installed, logged at debug. Nothing is asked of a
player, and a failure to reach GitHub is only logged.

## How it is made

**The game** (`client/src/update.rs`, natively only):

- `update::on_launch()` is the first thing `main` does. When the installer, the updater or the
  uninstaller runs the game with their own arguments, it does what they ask and exits; on a normal
  start it puts in place an update downloaded the time before.
- `UpdatePlugin` starts a thread that makes an `UpdateManager` on a `GithubSource` for
  `joko-games/star-racer` (prereleases included, the release candidates being named so), checks,
  and downloads. The thread hands the version found to the first screen through a shared slot;
  RESTART calls `apply_updates_and_restart`.
- The `velopack` crate is pinned (`=1.3.39-g8d2844f`) to the `vpk` tool that packs the release,
  so the package and the game reading it always agree.

**The release** (`.github/workflows/release.yml`, on the Windows runner, after the client is built
and zipped): `dotnet tool install --global vpk` at that same version, then `vpk pack` with the
package folder (the program, the license, the font's), the version from the tag without its `v`,
the id `StarRacer`, the title and authors "Star Racer", the `.ico` and the logo as the splash. Its
output goes into the release beside the zips:

| File | What it is |
| --- | --- |
| `StarRacer-win-Setup.exe` | the installer |
| `StarRacer-<version>-full.nupkg` | the whole version, which the updater downloads |
| `releases.win.json`, `assets.win.json`, `RELEASES` | the feed the installed games read |
| `StarRacer-win-Portable.zip` | the same, to run without installing |

A release without the installer or the feed is not published: the release job checks for them as
it does for the Windows and Linux zips. Updates are full packages, about the size of the zip; no
delta packages are made, which would need the previous release's files at packing time.

## Numbering the versions

The updater takes a release as newer by comparing versions as semver does, and semver compares the
words of a prerelease as text: `1.0.0-rc17` came before `1.0.0-rc8`, `1` sorting before `8`. So
the game installed at rc8 on 2026-11-24 never took rc10 to rc17, finding each older than itself
(the user's report on 2026-11-25: "Je suis en rc8 installée ici et ca n'update pas"; its updater's
log showed rc7 to rc8 done, and nothing after). Asked, the user chose to keep release candidates,
so since **v1.0.0-rc.18** they are numbered `1.0.0-rc.N`, the number behind a dot, which semver
compares as a number: rc.18, rc.19, ..., rc.100 come in order. A test holds the crates to it
(`the_version_orders_as_the_updater_reads_it`). The same text rule makes `rc.18` older than `rc8`
and `rc17`, so a game installed at rc7, rc8 or rc17 must be installed again by hand once, from the
installer of rc.18 or later; after that it updates itself. The final `1.0.0` comes after every
release candidate, whatever its number.

## Verified

v1.0.0-rc7 was the first release with the installer (2026-11-24, Windows, Linux and macOS all
built): the user downloaded `StarRacer-win-Setup.exe` from the release, installed the game and ran
it ("C'est ok"). An update has not been seen yet: it needs a release after one installed.

## Not done

- **Signing.** The installer and the game are unsigned, so SmartScreen asks before the first run
  ("More info", then "Run anyway"). Signing needs a code signing certificate; `vpk pack` takes one
  with `--signParams` when there is one.
- **Linux and macOS.** Velopack can package them too (an AppImage, a `.pkg`); they are still plain
  archives, without updates.
- **The first release with it** has nothing to update from: a game installed from it updates from
  the next one on.
