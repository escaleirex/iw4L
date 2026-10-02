param([string]$Device = $env:NATIVE_COD_DEVICE)
if (!$Device) { $Device = '100.90.96.7:36259' }
adb -s $Device logcat 'NativeCod:V' 'RustStdoutStderr:V' 'ReactNativeJS:V' 'AndroidRuntime:E' '*:S'
