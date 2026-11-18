<#
.SYNOPSIS
  Installs everything needed to work on Rust products on a fresh Windows machine.

.DESCRIPTION
  Windows twin of bootstrap.sh, with the same groups: base, rust, cargo-tools, gui, node,
  python, github, packaging and the optional editor. Packages come from winget; Rust tools go
  into ~/.cargo.

  Safe to run again: installed packages and tools are skipped. cargo-bk is installed from the
  clone this script lives in, so the bootstrap and `cargo bk` always come from the same tag.

  bootstrap.sh started from Git Bash hands over to this script. Through -File, -Without arrives
  as one comma-joined string, so every value is split on commas.

.PARAMETER List
  Show the groups and what they install.

.PARAMETER DryRun
  Print what would be installed.

.PARAMETER Yes
  No questions.

.PARAMETER Minimal
  base, rust and cargo-tools only.

.PARAMETER WithEditor
  Add the optional group: editor (VS Code and its extensions).

.PARAMETER Without
  Leave out groups. Accepts several values or one comma-separated string.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File bootstrap.ps1
  powershell -ExecutionPolicy Bypass -File bootstrap.ps1 -Minimal -DryRun
  powershell -ExecutionPolicy Bypass -File bootstrap.ps1 -Without gui,node -WithEditor
#>
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '', Justification = 'An interactive installer talks to the person running it.')]
[CmdletBinding()]
param(
    [switch]$List,
    [switch]$DryRun,
    [switch]$Yes,
    [switch]$Minimal,
    [switch]$WithEditor,
    [string[]]$Without = @()
)

$ErrorActionPreference = 'Stop'

$BuildToolsOverride = '--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
$RustComponents = @('rustfmt', 'clippy', 'llvm-tools-preview')
$EditorExtensions = @('rust-lang.rust-analyzer', 'vadimcn.vscode-lldb', 'tamasfe.even-better-toml')

# winget package ids per group. Groups without an entry install through the special steps below.
$Packages = [ordered]@{
    'base'        = @('Git.Git', 'Microsoft.VisualStudio.2022.BuildTools')
    'rust'        = @('Rustlang.Rustup')
    'cargo-tools' = @()
    'gui'         = @()
    'node'        = @('OpenJS.NodeJS.LTS')
    'python'      = @('Python.Python.3.12')
    'github'      = @('GitHub.cli')
    'packaging'   = @('JRSoftware.InnoSetup')
    'editor'      = @('Microsoft.VisualStudioCode')
}
$Special = @{
    'base'        = '(Build Tools with the C++ workload: the MSVC linker Rust needs)'
    'rust'        = 'stable with rustfmt clippy llvm-tools-preview, cargo-binstall'
    'cargo-tools' = 'cargo-bk (from this clone), cargo-deny, cargo-llvm-cov, cargo-about'
    'gui'         = '(nothing extra on Windows)'
    'editor'      = "VS Code extensions: $($EditorExtensions -join ' ')"
}
$DefaultGroups = @('base', 'rust', 'cargo-tools', 'gui', 'node', 'python', 'github', 'packaging')
$MinimalGroups = @('base', 'rust', 'cargo-tools')
$AllGroups = @($Packages.Keys)

if ($List) {
    foreach ($g in $AllGroups) {
        $text = (@($Packages[$g]) + @($Special[$g]) | Where-Object { $_ }) -join ' '
        '{0,-12} {1}' -f $g, $text
    }
    return
}

$skipped = @($Without | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
foreach ($g in $skipped) {
    if ($AllGroups -notcontains $g) { throw "Unknown group: $g" }
}

$groups = if ($Minimal) { $MinimalGroups } else { $DefaultGroups }
if ($WithEditor) { $groups += 'editor' }
$selected = @($groups | Where-Object { $skipped -notcontains $_ })

function Say([string]$Message) { Write-Host "`n==> $Message" -ForegroundColor Cyan }
function Warn([string]$Message) { Write-Warning $Message }

function Format-Command([string]$Exe, [string[]]$CmdArgs) {
    $shown = foreach ($a in $CmdArgs) { if ($a -match '\s') { '"' + $a + '"' } else { $a } }
    return ((@($Exe) + @($shown)) -join ' ')
}

# Run <exe> <args[]>: prints instead of running under -DryRun. Arguments stay separate, so a
# path with a space reaches the tool as one argument.
function Run([string]$Exe, [string[]]$CmdArgs = @()) {
    if ($DryRun) { Write-Host "[dry-run] $(Format-Command $Exe $CmdArgs)"; return }
    & $Exe @CmdArgs
}

# Test-Tool <exe> <args[]>: the tool ran and exited 0. Output is dropped.
function Test-Tool([string]$Exe, [string[]]$CmdArgs = @('--version')) {
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $global:LASTEXITCODE = 0
        & $Exe @CmdArgs 2>&1 | Out-Null
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    } finally {
        $ErrorActionPreference = $previous
    }
}

# Tools installed by winget or rustup are not on this session's PATH yet.
function Initialize-SessionPath {
    $parts = @()
    if ($env:OS -eq 'Windows_NT') {
        foreach ($scope in 'Machine', 'User') {
            $value = [Environment]::GetEnvironmentVariable('Path', $scope)
            if ($value) { $parts += $value }
        }
    }
    $parts += $env:Path
    $parts += (Join-Path (Join-Path $HOME '.cargo') 'bin')
    $env:Path = ($parts -join [IO.Path]::PathSeparator)
}

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "winget is missing. Install 'App Installer' from the Microsoft Store, then run this script again."
}

Say "Windows. Groups: $($selected -join ' ')"
if (-not $Yes -and -not $DryRun) {
    $answer = Read-Host 'Continue? [Y/n]'
    if ($answer -and $answer -notmatch '^[Yy]') { return }
}
Initialize-SessionPath

# --- winget ---------------------------------------------------------------------------------
$wanted = @($selected | ForEach-Object { $Packages[$_] } | Where-Object { $_ })
foreach ($id in $wanted) {
    $listed = winget list --id $id --exact --accept-source-agreements 2>$null | Select-String -SimpleMatch $id
    if ($listed) { Write-Host "  $id already installed"; continue }
    Say "Installing $id"
    $installArgs = @('install', '--id', $id, '--exact', '--silent', '--accept-package-agreements', '--accept-source-agreements')
    if ($id -eq 'Microsoft.VisualStudio.2022.BuildTools') { $installArgs += @('--override', $BuildToolsOverride) }
    Run 'winget' $installArgs
    if (-not $DryRun -and $LASTEXITCODE -ne 0) { Warn "winget could not install $id (exit $LASTEXITCODE)" }
}
Initialize-SessionPath

# --- Rust -----------------------------------------------------------------------------------
function Install-RustComponent {
    $haveRustup = Test-Tool 'rustup'
    if (-not $haveRustup -and -not $DryRun) {
        Warn 'rustup not found: open a new terminal after the winget step and run this again'
        return
    }
    $toolchains = if ($haveRustup) { @(rustup toolchain list 2>$null) } else { @() }
    if (-not ($toolchains -match '^stable')) {
        Say 'Installing the stable toolchain'
        Run 'rustup' (@('toolchain', 'install', 'stable', '--profile', 'minimal') + @($RustComponents | ForEach-Object { '-c'; $_ }))
        Run 'rustup' @('default', 'stable')
        return
    }
    $installed = @(rustup component list --installed 2>$null)
    foreach ($c in $RustComponents) {
        # The listing says "llvm-tools-<triple>" for the component added as llvm-tools-preview.
        $prefix = $c -replace '-preview$', ''
        if (-not ($installed -match "^$([regex]::Escape($prefix))")) {
            Say "Adding the rustup component $c"
            Run 'rustup' @('component', 'add', $c)
        }
    }
}

function Install-Binstall {
    if (Test-Tool 'cargo' @('binstall', '-V')) { return }
    Say 'Installing cargo-binstall'
    Run 'cargo' @('install', '--locked', 'cargo-binstall')
}

if ($selected -contains 'rust') {
    Install-RustComponent
    if ($DryRun -or (Test-Tool 'cargo')) { Install-Binstall }
}

# --- cargo tools ----------------------------------------------------------------------------
# Install-CargoTool <crate> <subcommand>: through binstall, falling back to a source build.
function Install-CargoTool([string]$Crate, [string]$Sub) {
    if (Test-Tool 'cargo' @($Sub, '--version')) { Say "$Crate already installed"; return }
    Say "Installing $Crate"
    if ($DryRun) {
        Write-Host "[dry-run] cargo binstall --no-confirm --locked $Crate || cargo install --locked $Crate"
        return
    }
    cargo binstall --no-confirm --locked $Crate
    if ($LASTEXITCODE -ne 0) { cargo install --locked $Crate }
}

function Install-CargoBk {
    $src = Join-Path (Join-Path (Join-Path $PSScriptRoot '..') 'crates') 'cargo-bk'
    if (-not (Test-Path -LiteralPath $src -PathType Container)) {
        Write-Host 'cargo-bk: skipped (not in this checkout)'
        return
    }
    $want = ''
    $manifest = Join-Path $src 'Cargo.toml'
    if (Test-Path -LiteralPath $manifest) {
        $line = Select-String -LiteralPath $manifest -Pattern '^version\s*=\s*"(.*)"' | Select-Object -First 1
        if ($line) { $want = $line.Matches[0].Groups[1].Value }
    }
    $have = ''
    if (Test-Tool 'cargo' @('bk', '--version')) { $have = [string](cargo bk --version 2>$null | Select-Object -First 1) }
    if ($have -and (-not $want -or $have.Contains($want))) { Say "cargo-bk already installed ($have)"; return }
    Say 'Installing cargo-bk from this clone'
    Run 'cargo' @('install', '--locked', '--path', $src)
}

if ($selected -contains 'cargo-tools') {
    if (-not $DryRun -and -not (Test-Tool 'cargo')) {
        Warn 'cargo not found: skipping cargo-tools (install the rust group first)'
    } else {
        Install-CargoBk
        Install-CargoTool 'cargo-deny' 'deny'
        Install-CargoTool 'cargo-llvm-cov' 'llvm-cov'
        Install-CargoTool 'cargo-about' 'about'
    }
}

# --- python version -------------------------------------------------------------------------
if (($selected -contains 'python') -and (Test-Tool 'python')) {
    $text = [string](python --version 2>$null | Select-Object -First 1)
    if ($text -match '(\d+)\.(\d+)' -and ([int]$Matches[1] -lt 3 -or ([int]$Matches[1] -eq 3 -and [int]$Matches[2] -lt 11))) {
        Warn "$text is older than 3.11; the release and manual tools need 3.11 or newer"
    }
}

# --- editor ---------------------------------------------------------------------------------
if ($selected -contains 'editor') {
    if (-not $DryRun -and -not (Test-Tool 'code')) {
        Warn 'VS Code (code) not found: open a new terminal after the winget step and run this again'
    } else {
        $present = if ($DryRun -and -not (Test-Tool 'code')) { @() } else { @(code --list-extensions 2>$null) }
        foreach ($ext in $EditorExtensions) {
            if ($present -contains $ext) { Say "$ext already installed" } else { Run 'code' @('--install-extension', $ext) }
        }
    }
}

# --- Summary --------------------------------------------------------------------------------
Say 'Installed versions (open a new terminal if a tool is missing here)'
$iscc = if (${env:ProgramFiles(x86)}) { Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' } else { '' }
$checks = [ordered]@{
    'git'         = { git --version }
    'rustup'      = { rustup --version }
    'rustc'       = { rustc --version }
    'cargo'       = { cargo --version }
    'cargo-bk'    = { cargo bk --version }
    'cargo-deny'  = { cargo deny --version }
    'llvm-cov'    = { cargo llvm-cov --version }
    'cargo-about' = { cargo about --version }
    'node'        = { node --version }
    'python'      = { python --version }
    'gh'          = { gh --version }
    'InnoSetup'   = { if ($iscc -and (Test-Path -LiteralPath $iscc)) { '6' } else { throw 'missing' } }
    'code'        = { code --version }
}
foreach ($name in $checks.Keys) {
    try {
        $ErrorActionPreference = 'Continue'
        $v = & $checks[$name] 2>$null | Select-Object -First 1
        if (-not $v -or $LASTEXITCODE -ne 0) { $v = '-' }
    } catch {
        $v = '-'
    } finally {
        $ErrorActionPreference = 'Stop'
    }
    '  {0,-14} {1}' -f $name, $v
}

Say 'Done. Open a new terminal so PATH changes (~/.cargo/bin) apply.'
# Task 14 adds the last step here: `cargo bk doctor`.
