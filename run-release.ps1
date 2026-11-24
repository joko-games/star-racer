# Runs Star Racer as run.ps1 does, built in release: every crate fully optimized, as players get it,
# for measuring frame times or playing at full speed. The release build lives in target/release,
# apart from the debug one, and takes about as long as the first debug build the first time; after
# that it rebuilds only what changed. Every option of run.ps1 and of the client works here.
#
#   ./run-release.ps1                                       build in release, then play
#   ./run-release.ps1 -SkipBuild                            play the release build already made
#   ./run-release.ps1 --guest --lobby Test --min-players 1  straight into a race, alone

# No param() block, as in run.ps1: client options such as `--guest` pass through untouched.
& (Join-Path $PSScriptRoot 'run.ps1') -Release @args
exit $LASTEXITCODE
