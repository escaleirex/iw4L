param([string]$Device = $env:NATIVE_COD_DEVICE)
$ErrorActionPreference = 'Stop'
if (!$Device) { $Device = '100.90.96.7:36259' }
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-21.0.10'
$env:ANDROID_HOME = 'C:\Users\Escal\AppData\Local\Android\Sdk'
$root = Split-Path $PSScriptRoot -Parent
Push-Location (Join-Path $root 'android')
try { .\gradlew.bat :app:installDebug --offline }
finally { Pop-Location }
if ($LASTEXITCODE) { throw 'installDebug failed.' }
adb connect $Device | Out-Null
adb -s $Device shell am start -n org.nativecod.app/.MainActivity
if ($LASTEXITCODE) { throw 'Launcher start failed.' }
