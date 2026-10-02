param([string]$Device = $env:NATIVE_COD_DEVICE)
if (!$Device) { $Device = '100.90.96.7:36259' }
adb -s $Device shell am start -n org.nativecod.app/.MainActivity
if ($LASTEXITCODE) { throw 'Launcher start failed.' }
