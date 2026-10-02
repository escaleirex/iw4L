param([switch]$Prepare)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if ($Prepare) {
        rustup target add aarch64-linux-android
        if ($LASTEXITCODE) { throw 'Rust target installation failed.' }
        cargo install cargo-ndk --version 4.1.2 --locked
        if ($LASTEXITCODE) { throw 'cargo-ndk installation failed.' }
        npm.cmd ci --no-audit --no-fund
        if ($LASTEXITCODE) { throw 'npm dependency installation failed.' }
    }
    & .\gradlew.bat assembleDebug
    if ($LASTEXITCODE) { throw 'Android build failed.' }
} finally { Pop-Location }
