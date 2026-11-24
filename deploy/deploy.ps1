# Builds Star Racer and puts it online, in one run: the game server for Linux, the web client for
# the browser, then the upload and the install. The machine must already be set up by
# the maintainer's setup-server.sh -- the star-racer user, the service, the Apache wiring. This only
# replaces the build. See docs/deploy.md.
#
# The "Deploy" GitHub workflow does the same thing on a push to main, from a runner, and that is
# the usual way a commit goes online. This script stays for the times that is not what is wanted:
# putting a build online without a commit, or when GitHub is having a bad day.
#
#   ./deploy/deploy.ps1 -Server root@host           the usual run: the machine is always named,
#                                                 never written here
#   ./deploy/deploy.ps1 -Server root@host -SkipBuild    upload what was built last time
#   ./deploy/deploy.ps1 -Server root@host -UploadOnly  stop once the archive is on the machine

param(
    # The machine, as user@host: asked for every time, never stored (since v1.0.0-rc1).
    [Parameter(Mandatory)] [string]$Server,
    [string]$Key = "$HOME\.ssh\id_ed25519",
    # Only used to fetch the page back at the end: where the game is served is the machine's
    # Apache configuration to decide, not this script's.
    [string]$WebPath = '',
    [switch]$SkipBuild,
    # Uploads the archive and stops there, without unpacking or installing it: for a run meant
    # only to carry the build over, the install being done by hand afterwards.
    [switch]$UploadOnly
)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

$stage = 'target/deploy'
$archive = 'target/deploy.tar.gz'

function Invoke-Checked {
    param([string]$What, [scriptblock]$Command)

    Write-Host "==> $What" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$What failed (exit code $LASTEXITCODE)"
    }
}

function Test-Tool {
    param([string]$Name)

    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

# The wasm-bindgen CLI must match the crate exactly, so the version is read from the lock file
# rather than written down here, where it would drift.
function Get-WasmBindgenVersion {
    $line = Select-String -Path 'Cargo.lock' -Pattern '^name = "wasm-bindgen"$' -Context 0, 1
    if (-not $line) {
        throw 'no wasm-bindgen in Cargo.lock'
    }
    return ($line.Context.PostContext[0] -split '"')[1]
}

# What it takes to build and upload. The Rust targets are added when missing, since that costs
# seconds; wasm-bindgen is a build of its own, so it is only reported.
if (-not $SkipBuild) {
    $wasmBindgenVersion = Get-WasmBindgenVersion
    foreach ($target in 'x86_64-unknown-linux-musl', 'wasm32-unknown-unknown') {
        if ((rustup target list --installed) -notcontains $target) {
            Invoke-Checked "adding the Rust target $target" { rustup target add $target }
        }
    }
    if (-not (Test-Tool 'wasm-bindgen')) {
        throw "wasm-bindgen is missing: cargo install wasm-bindgen-cli --version $wasmBindgenVersion --locked"
    }
    $installed = (wasm-bindgen --version) -split ' ' | Select-Object -Last 1
    if ($installed -ne $wasmBindgenVersion) {
        throw "wasm-bindgen $installed does not match the $wasmBindgenVersion in Cargo.lock: cargo install wasm-bindgen-cli --version $wasmBindgenVersion --locked"
    }
}
foreach ($tool in 'ssh', 'scp', 'tar') {
    if (-not (Test-Tool $tool)) {
        throw "$tool is missing from the PATH"
    }
}
if (-not (Test-Path $Key)) {
    throw "no SSH key at $Key (pass -Key)"
}

if (-not $SkipBuild) {
    # Both builds name a single crate, which is what the project otherwise avoids: each one has a
    # target of its own, with a cache of its own, so neither disturbs the native build.
    Invoke-Checked 'building the game server for Linux' {
        cargo build --locked --release -p star-racer-server --target x86_64-unknown-linux-musl
    }
    Invoke-Checked 'building the web client' {
        # The `web` profile: built for size, since the module is downloaded before the first frame.
        cargo build --locked --profile web -p star-racer-client --target wasm32-unknown-unknown
    }
    # The whole workspace, as run-release.ps1 builds it, so the two share one release cache. Only
    # the Windows server is taken from it: since v1.0.0-rc6 the Windows game is offered as the
    # installer of the latest GitHub release (docs/installer.md), not from this site.
    Invoke-Checked 'building the Windows server offered for download' {
        cargo build --locked --release --workspace
    }
}

Write-Host '==> staging what goes to the machine' -ForegroundColor Cyan
if (Test-Path $stage) {
    Remove-Item $stage -Recurse -Force
}
# The site at the root (site/, docs/deploy.md#the-site), the game inside it at play/ (since v0.1.7).
$game = "$stage/web/play"
New-Item -ItemType Directory -Path $stage, "$stage/web", $game, "$stage/web/downloads" | Out-Null
Copy-Item 'deploy/install.sh' $stage
Copy-Item 'target/x86_64-unknown-linux-musl/release/star-racer-server' $stage
Copy-Item 'server/data' "$stage/data" -Recurse
Copy-Item 'site/*' "$stage/web" -Recurse
Copy-Item 'client/web/*' $game
# No video since v0.1.14: the trailer and the loop behind the title were taken off the site, to be
# made again (docs/film.md#the-trailer); the site shows its poster.
# The game server for whoever wants their own (since v0.1.14, download.html#server): the program,
# its data folder beside it, where it looks when given none, its README and the license. For
# Windows from the release build above, for Linux the same static binary the machine runs.
foreach ($system in 'windows', 'linux') {
    $serverPackage = "$stage/package/star-racer-server"
    New-Item -ItemType Directory -Path $serverPackage | Out-Null
    if ($system -eq 'windows') {
        Copy-Item 'target/release/star-racer-server.exe' $serverPackage
    }
    else {
        Copy-Item 'target/x86_64-unknown-linux-musl/release/star-racer-server' $serverPackage
    }
    Copy-Item 'server/data' "$serverPackage/data" -Recurse
    Copy-Item 'LICENSE' $serverPackage
    Copy-Item 'deploy/server-readme.txt' "$serverPackage/README.txt"
    if ($system -eq 'windows') {
        Compress-Archive -Path $serverPackage -DestinationPath "$stage/web/downloads/star-racer-server-windows-x86_64.zip"
    }
    else {
        # Windows' tar records every file as 0666, so the program would not run once unpacked. An
        # mtree list states each entry's mode instead (libarchive reads it with `@`), its paths
        # relative to the folder it is run from. Windows' own tar, libarchive: a GNU tar earlier in
        # the PATH (Git's) reads no mtree.
        $bsdtar = Join-Path $env:SystemRoot 'System32\tar.exe'
        $root = (Resolve-Path "$stage/package").Path
        $entries = @('#mtree')
        foreach ($item in Get-ChildItem $root -Recurse) {
            $path = $item.FullName.Substring($root.Length + 1).Replace('\', '/')
            $entries += if ($item.PSIsContainer) {
                "$path type=dir mode=0755 uid=0 gid=0"
            }
            elseif ($path -eq 'star-racer-server/star-racer-server') {
                "$path type=file mode=0755 uid=0 gid=0"
            }
            else {
                "$path type=file mode=0644 uid=0 gid=0"
            }
        }
        $list = Join-Path $root '..\server.mtree'
        Set-Content $list ($entries -join "`n") -NoNewline
        $tarball = (Join-Path (Resolve-Path "$stage/web/downloads").Path 'star-racer-server-linux-x86_64.tar.gz')
        Push-Location $root
        try {
            Invoke-Checked 'packing the Linux server' { & $bsdtar -czf $tarball "@$list" }
        }
        finally {
            Pop-Location
        }
        Remove-Item $list
    }
    Remove-Item "$stage/package" -Recurse -Force
}

Invoke-Checked 'generating the JavaScript bindings' {
    wasm-bindgen --target web --no-typescript --out-dir $game --out-name star-racer-client `
        target/wasm32-unknown-unknown/web/star-racer-client.wasm
}
# Shrinks the module further. Not having it is no reason to stop, but the download is longer.
$module = "$game/star-racer-client_bg.wasm"
if (Test-Tool 'wasm-opt') {
    $before = (Get-Item $module).Length / 1MB
    # The profile strips the section that says which WebAssembly features the module uses, so
    # they are named here: the ones Rust turns on for this target. Without them wasm-opt refuses
    # the module's own instructions.
    $features = '--enable-bulk-memory', '--enable-sign-ext', '--enable-nontrapping-float-to-int',
        '--enable-mutable-globals', '--enable-reference-types', '--enable-multivalue'
    Invoke-Checked 'shrinking the module' { wasm-opt -Oz @features $module -o "$module.opt" }
    Move-Item "$module.opt" $module -Force
    $after = (Get-Item $module).Length / 1MB
    Write-Host ("    {0:n1} MB, down from {1:n1} MB" -f $after, $before)
}
else {
    Write-Warning 'wasm-opt is missing: the module ships unshrunk (cargo install wasm-opt)'
}

Invoke-Checked 'packing' { tar -czf $archive -C $stage . }
Invoke-Checked "uploading to $Server" {
    scp -i $Key -o BatchMode=yes $archive "${Server}:/tmp/star-racer-deploy.tar.gz"
}
if ($UploadOnly) {
    Write-Host '==> uploaded, not installed' -ForegroundColor Cyan
    Write-Host "    /tmp/star-racer-deploy.tar.gz is on ${Server}: unpack it and run install.sh from inside it"
    return
}

# A here-string has the line endings of this file, which is CRLF wherever Git checks it out for
# Windows. The remote shell would take every `\r` as part of the line: `set -e\r` is refused, so
# nothing stops on error, `cd` lands in a directory named `star-racer-deploy\r`, `install.sh\r` does
# not exist, and the last `rm -f` succeeds -- a deployment that installed nothing and reported
# success. So the script goes over with plain newlines, whatever this file has.
$install = @'
set -e
rm -rf /tmp/star-racer-deploy
mkdir -p /tmp/star-racer-deploy
tar -xzf /tmp/star-racer-deploy.tar.gz -C /tmp/star-racer-deploy
cd /tmp/star-racer-deploy
sh install.sh
rm -rf /tmp/star-racer-deploy /tmp/star-racer-deploy.tar.gz
'@ -replace "`r", ''
Invoke-Checked 'installing' {
    ssh -i $Key -o BatchMode=yes $Server $install
}

# The page is fetched back, so a deployment that cannot be played does not pass for a good one.
$hostName = $Server.Split('@')[-1]
$url = "https://$hostName$WebPath/"
Write-Host "==> checking $url" -ForegroundColor Cyan
$page = Invoke-WebRequest $url -UseBasicParsing
$served = Invoke-WebRequest "$url`play/star-racer-client_bg.wasm" -Method Head -UseBasicParsing
$download = Invoke-WebRequest "$url`downloads/star-racer-server-windows-x86_64.zip" -Method Head -UseBasicParsing
# A compressed answer carries no length, so the size is only reported when the server gives one.
$length = $served.Headers['Content-Length'] | Select-Object -First 1
$size = if ($length) { ', {0:n1} MB' -f ($length / 1MB) } else { '' }
$type = $served.Headers['Content-Type'] | Select-Object -First 1
Write-Host "    site $($page.StatusCode), game module $($served.StatusCode) ($type$size), Windows server download $($download.StatusCode)"
Write-Host "Star Racer is live at $url" -ForegroundColor Green
