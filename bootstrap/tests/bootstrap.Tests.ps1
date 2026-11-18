# Pester 5 tests for bootstrap.ps1. winget, rustup, cargo and code are replaced by functions
# that record their arguments in $global:Calls and act "installed" or "missing" according to
# $global:StubMissing. Nothing here calls the real tools.
#
# Usage: Invoke-Pester bootstrap/tests

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidGlobalVars', '', Justification = 'The stubs share state with the script under test through the global scope.')]
param()

BeforeAll {
    $script:Bootstrap = Join-Path $PSScriptRoot '../bootstrap.ps1'

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
            if (Test-StubMissing $sub[$key]) { $global:LASTEXITCODE = 1 }
            elseif ($key -eq 'bk') { if ($global:BkVersion) { $global:BkVersion } else { '1.0.0' } }
            else { "$key 1.0.0" }
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
        param([string[]]$Missing = @(), [hashtable]$Params = @{}, [string]$Script = $script:Bootstrap, [string]$BkVersion = '')
        $global:Calls = @()
        $global:CallArgs = @()
        $global:StubMissing = $Missing
        $global:BkVersion = $BkVersion
        $path = $env:Path
        try {
            $out = & $Script @Params *>&1 | Out-String
        } finally {
            $env:Path = $path
        }
        return $out
    }
}

AfterAll {
    foreach ($f in 'winget', 'rustup', 'cargo', 'code', 'Write-Call', 'Test-StubMissing', 'Invoke-Bootstrap') {
        Remove-Item "Function:\$f" -ErrorAction SilentlyContinue
    }
    Remove-Variable Calls, CallArgs, StubMissing, BkVersion -Scope Global -ErrorAction SilentlyContinue
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
            # A copy without crates/ keeps the real cargo-bk crate out of it.
            $dir = Join-Path ([IO.Path]::GetTempPath()) ('bk nocrates ' + [guid]::NewGuid())
            New-Item -ItemType Directory -Path (Join-Path $dir 'bootstrap') -Force | Out-Null
            Copy-Item $script:Bootstrap (Join-Path $dir 'bootstrap/bootstrap.ps1')
            try {
                $out = Invoke-Bootstrap -Params @{ Yes = $true; WithEditor = $true } -Script (Join-Path $dir 'bootstrap/bootstrap.ps1')
            } finally {
                Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
            }
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
                New-Item -ItemType Directory -Path (Join-Path $dir 'bootstrap'), (Join-Path $dir 'crates/cargo-bk') -Force | Out-Null
                Copy-Item $script:Bootstrap (Join-Path $dir 'bootstrap/bootstrap.ps1')
                Set-Content (Join-Path $dir 'crates/cargo-bk/Cargo.toml') "[package]`nname = `"cargo-bk`"`nversion = `"0.1.0`"`n"
                Invoke-Bootstrap -Missing 'cargo-bk' -Params @{ Yes = $true } -Script (Join-Path $dir 'bootstrap/bootstrap.ps1') | Out-Null
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

    Context 'cargo-bk version against the clone tag' {
        BeforeAll {
            $script:Clone = Join-Path ([IO.Path]::GetTempPath()) ('bk clone ' + [guid]::NewGuid())
            New-Item -ItemType Directory -Path (Join-Path $script:Clone 'bootstrap'), (Join-Path $script:Clone 'crates/cargo-bk') -Force | Out-Null
            Copy-Item $script:Bootstrap (Join-Path $script:Clone 'bootstrap/bootstrap.ps1')
            Set-Content (Join-Path $script:Clone 'crates/cargo-bk/Cargo.toml') "[package]`nname = `"cargo-bk`"`nversion = `"0.1.0-dev`"`n"
            $script:CloneScript = Join-Path $script:Clone 'bootstrap/bootstrap.ps1'
            $g = @('-C', $script:Clone, '-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'commit.gpgsign=false')
            git @g init -q | Out-Null
            git @g add -A | Out-Null
            git @g commit -q -m one | Out-Null
            git @g tag v1.2.3 | Out-Null
            $script:G = $g
        }
        AfterAll {
            Remove-Item $script:Clone -Recurse -Force -ErrorAction SilentlyContinue
        }
        It 'skips the install when the version equals the tag' {
            Invoke-Bootstrap -Params @{ Yes = $true } -Script $script:CloneScript -BkVersion '1.2.3' | Out-Null
            $global:Calls | Where-Object { $_ -match '^cargo install --locked --path' } | Should -BeNullOrEmpty
        }
        It 'installs when the version differs from the tag' {
            Invoke-Bootstrap -Params @{ Yes = $true } -Script $script:CloneScript -BkVersion '1.0.0' | Out-Null
            $global:Calls | Where-Object { $_ -match '^cargo install --locked --path' } | Should -Not -BeNullOrEmpty
        }
        It 'still installs when git fails with a message on stderr' {
            function global:git { Write-Error 'fatal: No names found'; $global:LASTEXITCODE = 128 }
            try {
                Invoke-Bootstrap -Params @{ Yes = $true } -Script $script:CloneScript -BkVersion '1.2.3' | Out-Null
            } finally {
                Remove-Item Function:\git -ErrorAction SilentlyContinue
            }
            $global:Calls | Where-Object { $_ -match '^cargo install --locked --path' } | Should -Not -BeNullOrEmpty
        }
        It 'always installs when HEAD is not exactly at a tag' {
            git @script:G commit -q --allow-empty -m two | Out-Null
            Invoke-Bootstrap -Params @{ Yes = $true } -Script $script:CloneScript -BkVersion '1.2.3' | Out-Null
            $global:Calls | Where-Object { $_ -match '^cargo install --locked --path' } | Should -Not -BeNullOrEmpty
        }
    }

    Context 'doctor as the last step' {
        BeforeAll {
            $script:Root = Join-Path ([IO.Path]::GetTempPath()) ('bk doctor ' + [guid]::NewGuid())
            $script:Sub = Join-Path $script:Root 'prod/sub'
            New-Item -ItemType Directory -Path $script:Sub -Force | Out-Null
            Set-Content (Join-Path $script:Root 'prod/Cargo.toml') "[workspace]`n[workspace.metadata.buildkit]`nversion = `"0.1.0`"`n"
        }
        AfterAll {
            Remove-Item $script:Root -Recurse -Force -ErrorAction SilentlyContinue
        }
        It 'runs cargo bk doctor inside a product' {
            Push-Location $script:Sub
            try { Invoke-Bootstrap -Params @{ Yes = $true } | Out-Null } finally { Pop-Location }
            $global:Calls | Should -Contain 'cargo bk doctor'
        }
        It 'runs cargo bk doctor --machine outside a product' {
            Push-Location $script:Root
            try { Invoke-Bootstrap -Params @{ Yes = $true } | Out-Null } finally { Pop-Location }
            $global:Calls | Should -Contain 'cargo bk doctor --machine'
        }
        It 'skips doctor when cargo-bk is not installed' {
            $out = Invoke-Bootstrap -Missing 'cargo-bk' -Params @{ Yes = $true; Without = @('cargo-tools') }
            $global:Calls | Where-Object { $_ -match '^cargo bk doctor' } | Should -BeNullOrEmpty
            $out | Should -Match 'doctor: skipped \(cargo-bk is not installed\)'
        }
        It 'does not run doctor under -DryRun' {
            Invoke-Bootstrap -Missing 'all' -Params @{ Yes = $true; DryRun = $true } | Out-Null
            $global:Calls | Where-Object { $_ -match '^cargo bk doctor' } | Should -BeNullOrEmpty
        }
    }
    Context 'PATH' {
        It 'does not grow when the script runs twice in one session' {
            $path = $env:Path
            try {
                $global:StubMissing = @()
                & $script:Bootstrap -Yes *>&1 | Out-Null
                $once = $env:Path
                & $script:Bootstrap -Yes *>&1 | Out-Null
                $env:Path | Should -Be $once
            } finally {
                $env:Path = $path
            }
        }
    }
}
