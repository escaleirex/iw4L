$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $sdk = $env:ANDROID_HOME
    if (!$sdk) { $sdk = $env:ANDROID_SDK_ROOT }
    if (!$sdk) { throw 'Set ANDROID_HOME to the Android SDK directory.' }
    $env:ANDROID_NDK_HOME = Join-Path $sdk 'ndk/28.2.13676358'
    cargo ndk -t arm64-v8a -P 31 check --locked -p android_runtime --features iw4-runtime
    if ($LASTEXITCODE) { throw 'IW4 Android compile audit failed; see docs/ANDROID_STATUS.md.' }
} finally { Pop-Location }
