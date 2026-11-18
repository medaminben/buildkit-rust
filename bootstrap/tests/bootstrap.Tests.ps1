# Pester 5 tests for bootstrap.ps1. winget, rustup, cargo and code are replaced by functions
# that record their arguments in $global:Calls and act "installed" or "missing" according to
# $global:StubMissing. Nothing here calls the real tools.
#
# Usage: Invoke-Pester bootstrap/tests

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidGlobalVars', '', Justification = 'The stubs share state with the script under test through the global scope.')]
param()

BeforeAll {
    $script:Bootstrap = Join-Path $PSScriptRoot '..' 'bootstrap.ps1'

    function global:Write-Call {
        param([string]$Name, [object[]]$CallArgs)
        $global:Calls += , ("$Name " + ($CallArgs -join ' ')).Trim()
        $global:CallArgs += , (, $Name + $CallArgs)
    }
    function global:Test-StubMissing {
        param([string]$Name)
        return ($global:StubMissing -contains $Name) -or ($global:StubMissing -contains 'all')
    }

    function global:winget {
        Write-Call 'winget' $args
        $global:LASTEXITCODE = 0
        if ($args[0] -eq 'list') {
            $id = $args[([array]::IndexOf($args, '--id') + 1)]
            if (-not (Test-StubMissing $id)) { "Name  $id  1.0" }
        }
    }
    function global:rustup {
        Write-Call 'rustup' $args
        $global:LASTEXITCODE = 0
        $line = $args -join ' '
        if ($line -eq '--version') {
            if (Test-StubMissing 'rustup') { $global:LASTEXITCODE = 1 } else { 'rustup 1.0.0' }
        } elseif ($line -eq 'toolchain list') {
            if (-not (Test-StubMissing 'toolchain')) { 'stable-x86_64-pc-windows-msvc (default)' }
        } elseif ($line -eq 'component list --installed') {
            'rustfmt-x86_64-pc-windows-msvc'
            'clippy-x86_64-pc-windows-msvc'
            if (-not (Test-StubMissing 'llvm-tools')) { 'llvm-tools-x86_64-pc-windows-msvc' }
        }
    }
    function global:cargo {
        Write-Call 'cargo' $args
        $global:LASTEXITCODE = 0
        $sub = @{ '--version' = 'cargo'; 'binstall' = 'binstall'; 'bk' = 'cargo-bk'; 'deny' = 'cargo-deny'
            'llvm-cov' = 'cargo-llvm-cov'; 'about' = 'cargo-about' }
        $key = $args[0]
        if ($sub.ContainsKey($key) -and ($args -contains '--version' -or $args -contains '-V')) {
            if (Test-StubMissing $sub[$key]) { $global:LASTEXITCODE = 1 } else { "$key 1.0.0" }
        }
    }
    function global:code {
        Write-Call 'code' $args
        $global:LASTEXITCODE = 0
        if ($args[0] -eq '--list-extensions' -and -not (Test-StubMissing 'extensions')) {
            'rust-lang.rust-analyzer'; 'vadimcn.vscode-lldb'; 'tamasfe.even-better-toml'
        }
    }

    # Run bootstrap.ps1 with the given stub state; returns the captured output as one string.
    function global:Invoke-Bootstrap {
        param([string[]]$Missing = @(), [hashtable]$Params = @{}, [string]$Script = $script:Bootstrap)
        $global:Calls = @()
        $global:CallArgs = @()
        $global:StubMissing = $Missing
        $out = & $Script @Params *>&1 | Out-String
        return $out
    }
}

AfterAll {
    foreach ($f in 'winget', 'rustup', 'cargo', 'code', 'Write-Call', 'Test-StubMissing', 'Invoke-Bootstrap') {
        Remove-Item "function:global:$f" -ErrorAction SilentlyContinue
    }
    Remove-Variable Calls, CallArgs, StubMissing -Scope Global -ErrorAction SilentlyContinue
}

Describe 'bootstrap.ps1' {
    It 'exists' {
        $script:Bootstrap | Should -Exist
    }

    Context '-List' {
        It 'names every group and installs nothing' {
            $out = Invoke-Bootstrap -Params @{ List = $true }
            foreach ($g in 'base', 'rust', 'cargo-tools', 'gui', 'node', 'python', 'github', 'packaging', 'editor') {
                $out | Should -Match "(?m)^$g( |$)"
            }
            $global:Calls | Where-Object { $_ -match '^winget install' } | Should -BeNullOrEmpty
        }
    }

    Context '-DryRun with everything missing' {
        BeforeAll {
            $script:Out = Invoke-Bootstrap -Missing 'all' -Params @{ DryRun = $true; Yes = $true; WithEditor = $true }
        }
        It 'makes no install calls' {
            $global:Calls | Where-Object { $_ -match '^winget install' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^rustup (toolchain install|component add|default)' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^cargo (install|binstall --no-confirm)' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^code --install-extension' } | Should -BeNullOrEmpty
        }
        It 'prints what it would install' {
            $script:Out | Should -Match '(?m)^\[dry-run\] winget install --id Git\.Git '
            $script:Out | Should -Match '(?m)^\[dry-run\] winget install --id Microsoft\.VisualStudio\.2022\.BuildTools .*--override "--quiet --wait --add Microsoft\.VisualStudio\.Workload\.VCTools --includeRecommended"'
            foreach ($id in 'Rustlang.Rustup', 'OpenJS.NodeJS.LTS', 'Python.Python.3.12', 'GitHub.cli', 'JRSoftware.InnoSetup', 'Microsoft.VisualStudioCode') {
                $script:Out | Should -Match ('(?m)^\[dry-run\] winget install --id ' + [regex]::Escape($id) + ' ')
            }
            $script:Out | Should -Match '(?m)^\[dry-run\] cargo install'
            $script:Out | Should -Match '(?m)^\[dry-run\] code --install-extension'
        }
    }

    Context 'nothing missing' {
        It 'is idempotent' {
            $out = Invoke-Bootstrap -Params @{ Yes = $true; WithEditor = $true }
            $global:Calls | Where-Object { $_ -match '^winget install' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^rustup (toolchain install|component add|default)' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^cargo (install|binstall --no-confirm)' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match '^code --install-extension' } | Should -BeNullOrEmpty
            $out | Should -Match 'cargo-bk: skipped \(not in this checkout\)'
        }
    }

    Context 'missing pieces' {
        It 'installs a missing winget package with the silent flags' {
            Invoke-Bootstrap -Missing 'Git.Git' -Params @{ Yes = $true } | Out-Null
            $global:Calls | Should -Contain 'winget install --id Git.Git --exact --silent --accept-package-agreements --accept-source-agreements'
        }
        It 'installs the stable toolchain with the pinned rustup arguments' {
            Invoke-Bootstrap -Missing 'toolchain' -Params @{ Yes = $true } | Out-Null
            $global:Calls | Should -Contain 'rustup toolchain install stable --profile minimal -c rustfmt -c clippy -c llvm-tools-preview'
            $global:Calls | Should -Contain 'rustup default stable'
        }
        It 'adds only the missing component' {
            Invoke-Bootstrap -Missing 'llvm-tools' -Params @{ Yes = $true } | Out-Null
            $global:Calls | Should -Contain 'rustup component add llvm-tools-preview'
            $global:Calls | Where-Object { $_ -match '^rustup component add .*(rustfmt|clippy)' } | Should -BeNullOrEmpty
        }
    }

    Context '-Without' {
        It 'drops a group' {
            Invoke-Bootstrap -Missing 'all' -Params @{ Yes = $true; Without = @('node') } | Out-Null
            $global:Calls | Where-Object { $_ -match 'OpenJS\.NodeJS\.LTS' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match 'Python\.Python' } | Should -Not -BeNullOrEmpty
        }
        It 'splits one comma-joined string, as it arrives through -File' {
            Invoke-Bootstrap -Missing 'all' -Params @{ Yes = $true; Without = @('gui,node') } | Out-Null
            $global:Calls | Where-Object { $_ -match 'OpenJS\.NodeJS\.LTS' } | Should -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match 'Python\.Python' } | Should -Not -BeNullOrEmpty
        }
        It 'accepts several values and mixed forms' {
            Invoke-Bootstrap -Missing 'all' -Params @{ Yes = $true; Without = @('node', 'python,github') } | Out-Null
            foreach ($id in 'OpenJS\.NodeJS\.LTS', 'Python\.Python', 'GitHub\.cli') {
                $global:Calls | Where-Object { $_ -match $id } | Should -BeNullOrEmpty
            }
            $global:Calls | Where-Object { $_ -match 'Git\.Git' } | Should -Not -BeNullOrEmpty
        }
        It 'rejects an unknown group' {
            { Invoke-Bootstrap -Params @{ Yes = $true; Without = @('nonsense') } } | Should -Throw '*Unknown group*'
        }
    }

    Context '-Minimal' {
        It 'keeps base, rust and cargo-tools only' {
            Invoke-Bootstrap -Missing 'all' -Params @{ Yes = $true; Minimal = $true } | Out-Null
            $global:Calls | Where-Object { $_ -match 'Git\.Git' } | Should -Not -BeNullOrEmpty
            $global:Calls | Where-Object { $_ -match 'OpenJS|Python|GitHub\.cli|InnoSetup' } | Should -BeNullOrEmpty
        }
    }

    Context 'a path with a space' {
        It 'passes --path to cargo install as one argument' {
            $dir = Join-Path ([IO.Path]::GetTempPath()) ("bk test " + [guid]::NewGuid())
            try {
                New-Item -ItemType Directory -Path (Join-Path $dir 'bootstrap'), (Join-Path $dir 'crates' 'cargo-bk') -Force | Out-Null
                Copy-Item $script:Bootstrap (Join-Path $dir 'bootstrap' 'bootstrap.ps1')
                Set-Content (Join-Path $dir 'crates' 'cargo-bk' 'Cargo.toml') "[package]`nname = `"cargo-bk`"`nversion = `"0.1.0`"`n"
                Invoke-Bootstrap -Missing 'cargo-bk' -Params @{ Yes = $true } -Script (Join-Path $dir 'bootstrap' 'bootstrap.ps1') | Out-Null
                $install = $global:CallArgs.Where({ $_[0] -eq 'cargo' -and $_[1] -eq 'install' -and $_ -contains '--path' })[0]
                $install | Should -Not -BeNullOrEmpty
                $path = $install[([array]::IndexOf($install, '--path') + 1)]
                $path | Should -Match 'bk test '
                $path | Should -Match 'cargo-bk$'
                $install | Should -Contain '--locked'
            } finally {
                Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}
