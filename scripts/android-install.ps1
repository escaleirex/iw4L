param([string]$Device = $env:NATIVE_COD_DEVICE)
$ErrorActionPreference = 'Stop'
if (!$Device) { $Device = '100.90.96.7:36259' }
adb connect $Device
if ($LASTEXITCODE) { throw 'ADB connection failed.' }
$apk = Join-Path (Split-Path $PSScriptRoot -Parent) 'android/app/build/outputs/apk/debug/app-debug.apk'
if (!(Test-Path -LiteralPath $apk)) { throw 'Build the APK with scripts/android-build.ps1 first.' }
adb -s $Device install -r $apk
if ($LASTEXITCODE) { throw 'APK installation failed.' }
