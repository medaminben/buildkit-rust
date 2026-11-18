<#
.SYNOPSIS
Sets up this product after a clone (Windows PowerShell 5.1 or later).

.DESCRIPTION
1. Fetches buildkit-rust's bootstrap at the pinned tag and runs it, when `cargo bk` is missing
   or on another version than Cargo.toml pins.
2. Nothing for system packages on Windows (they are an apt list).
3. npm ci (commit hooks) and cargo fetch.
4. cargo bk doctor, then cargo build and cargo test.

.PARAMETER Yes
Run the bootstrap without questions.

.PARAMETER DryRun
Print the steps instead of running them.
#>
[CmdletBinding()]
param(
    [switch]$Yes,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$BootstrapRepo = 'https://github.com/medaminben/buildkit-rust'

Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)

function Invoke-Step {
    param([string]$Program, [string[]]$Arguments)
    if ($DryRun) {
        Write-Host ('+ ' + $Program + ' ' + ($Arguments -join ' '))
        return
    }
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

# The pinned tag: the first `version = "` line inside [workspace.metadata.buildkit].
$pin = $null
$inSection = $false
foreach ($line in Get-Content -LiteralPath 'Cargo.toml') {
    if ($line -match '^\[') { $inSection = ($line.Trim() -eq '[workspace.metadata.buildkit]') }
    if ($inSection -and $line -match '^version = "([^"]*)"') { $pin = $Matches[1]; break }
}
if (-not $pin) { throw 'No version in [workspace.metadata.buildkit] of Cargo.toml.' }

Write-Host "== 1/4 cargo bk v$pin"
$installed = $null
try {
    $text = (& cargo bk --version 2>$null) -join ' '
    if ($text -match '(\d+\.\d+\.\d+)') { $installed = $Matches[1] }
} catch {
    $installed = $null
}
if ($installed -ne $pin) {
    $tools = "build/tools/buildkit-rust-v$pin"
    if (-not (Test-Path -LiteralPath $tools)) {
        Invoke-Step 'git' @('clone', '--depth', '1', '--branch', "v$pin", $BootstrapRepo, $tools)
    }
    $bootstrapArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "$tools/bootstrap/bootstrap.ps1")
    if ($Yes) { $bootstrapArgs += '-Yes' }
    Invoke-Step 'powershell.exe' $bootstrapArgs
    $env:Path = "$HOME\.cargo\bin;$env:Path"
}

Write-Host '== 2/4 system packages'
Write-Host 'Nothing to do on Windows.'

Write-Host '== 3/4 commit hooks and dependencies'
if (Test-Path -LiteralPath 'package-lock.json') {
    Invoke-Step 'npm' @('ci')
} else {
    Invoke-Step 'npm' @('install')
}
if (Test-Path -LiteralPath 'Cargo.lock') {
    Invoke-Step 'cargo' @('fetch', '--locked')
} else {
    Invoke-Step 'cargo' @('fetch')
}

Write-Host '== 4/4 doctor, build, test'
Invoke-Step 'cargo' @('bk', 'doctor')
Invoke-Step 'cargo' @('build')
Invoke-Step 'cargo' @('test')
Write-Host 'Setup done.'
