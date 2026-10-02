# Android development

Upstream: `vladtrc/iw4L`, kept current by the sync below.
The Windows baseline compiled before engine modifications. Asset-dependent
desktop scenarios were not run because no desktop game installation was configured.

## Prerequisites

Rust 1.97.1, Node 22.22.2+, npm, JDK 21, Android SDK platform/build-tools 36,
NDK 28.2.13676358 and ADB. Gradle 9.4.1, AGP 9.2.1, Kotlin 2.2.0,
React Native 0.87.1 and Bevy 0.19 are pinned by the project and lockfiles.
Set `JAVA_HOME` and `ANDROID_HOME`; alternatively set `sdk.dir` in
`android/local.properties` (machine-local, ignored).

```powershell
rustup target add aarch64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
npm.cmd ci
.\gradlew.bat assembleDebug
.\scripts\android-install.ps1 -Device 100.90.96.7:36259
.\scripts\android-run.ps1
.\scripts\android-log.ps1
```

APK: `android/app/build/outputs/apk/debug/app-debug.apk`, ARM64 only, API 31+.
The debug APK includes the Hermes bundle and requires no Metro server.
`scripts/android-build.ps1 -Prepare` also installs Rust/npm prerequisites.
Wireless debugging ports may change; scripts accept `-Device` or
`NATIVE_COD_DEVICE`. Pair using `adb pair` if Android requests a new pairing.

## Game files

Open MW2 → Select Game Installation and select the extracted installation
folder using Android's document picker. A full installation is accepted;
only Multiplayer is supported by this development slice. Access is read-only
and persisted. An executable is not required. Revoked permission needs reselection.
Missing or incompatible FastFiles produce specific errors. No game data is downloaded.

PLAY opens the native parser proof; Check Vulkan opens a data-free surface.
App configuration/reports live in `filesDir`; provider import cache in `cacheDir`.
There is no raw-path conversion or broad storage permission.
Runtime process `:iw4` does not initialize React Native; Back returns to the launcher.

## Current limitations

See [ANDROID_STATUS.md](ANDROID_STATUS.md) for measured status and remaining blockers.
`scripts/android-check.ps1` audits the optional full IW4 bootstrap on Android.
The parser proof uses the shared transport and original FastFile parser.
It does not claim map rendering, gameplay, controller gameplay or audio support.

## Upstream sync

This fork's `master` is upstream `master` plus the Android runtime.
`.github/workflows/sync-upstream.yml` merges `vladtrc/iw4L` `master` daily
(and on demand from the Actions tab), compiles `android_runtime` for
`aarch64-linux-android`, and pushes only if that passes. A conflict or a
compile error leaves `master` untouched and opens an issue named
"Upstream sync needs a hand". The check is a compile check; it does not run
the app on a device.

Android-only code lives in files of its own (`crates/android_runtime`,
`crates/assets/src/lane/iw4_view.rs`,
`crates/render_gpu/src/drawsurf/exact_pipeline_spirv.rs`) so upstream edits
rarely touch the same lines. To merge by hand:

```powershell
git fetch upstream
git merge upstream/master
```
