# Android status

| Component | Status | Notes |
|---|---|---|
| Desktop baseline | Working | Windows dev build completed before port changes |
| IW4 Android compile | Working | Runtime linked and started; match scene not captured |
| ARM64 build | Working | Debug APK links the IW4 runtime (`iw4-runtime`) |
| Android activity | Working | Launcher in the main process; GameActivity in `:iw4` |
| Vulkan | Working | Diagnostic scene on Adreno 740; match scene not captured |
| Launcher | Working | Hermes bundle, no Metro; D-pad, touch, and Back exercised |
| SAF filesystem | Working | Read-only tree permission; parser proof read both FastFiles |
| FastFile | Working | `common_mp.ff` and a map zone walk to the end through SAF |
| Map rendering | In progress | PLAY starts the desktop match loader; device dropped during menu catalog |
| Audio | Not started | External ffmpeg conversion needs Android replacement |
| Gamepad launcher | Working | D-pad moves focus; A/B and touch open MW2; first key after a cold start can be ignored |
| Gamepad gameplay | In progress | Retroid axes/buttons are injected as a Bevy gamepad; not exercised in a match |
| Networking | Not tested | No protocol compatibility work |

Upstream commit: `bfe3c2aa700c06285c12c7cd659e3a5f42468f12`.
Hardware: Retroid Pocket 6, Android 13, ARM64; ADB access confirmed.

Remaining integration work:

- Finish the `mp_boneyard` load that started on device and capture a frame.
- Route IWD reads through `GameDataSource` so PLAY does not need all-files access.
- Audit renderer requirements against Adreno Vulkan features and limits.
- Restore portable audio. Confirm the injected gamepad inside a running match.
- Verify full map installation, player and bots before marking gameplay working.
