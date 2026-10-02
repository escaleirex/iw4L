use std::io::{self, Read};
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use asset_transport::data_source::{
    GameDataEntry, GameDataFile, GameDataMetadata, GameDataReader, GameDataSource,
};
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    renderer::{RenderAdapter, RenderAdapterInfo},
    settings::{Backends, RenderCreation, WgpuSettings},
};
use bevy::window::AppLifecycle;
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JString, JValue},
    sys::{jboolean, jfloat, jint},
};
use serde::Deserialize;

struct LaunchConfig {
    source: Option<Arc<AndroidSafDataSource>>,
    root: Option<PathBuf>,
    files: PathBuf,
    diagnostic: bool,
}
static CONFIG: OnceLock<LaunchConfig> = OnceLock::new();
static EXIT: AtomicBool = AtomicBool::new(false);

pub struct AndroidSafDataSource {
    vm: JavaVM,
    object: GlobalRef,
    index: OnceLock<Vec<GameDataEntry>>,
}

#[derive(Deserialize)]
struct DocumentEntry {
    path: String,
    directory: bool,
    size: i64,
    modified: u64,
}

fn jni_io(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

impl AndroidSafDataSource {
    fn new(env: &mut JNIEnv, object: JObject) -> io::Result<Self> {
        Ok(Self {
            vm: env.get_java_vm().map_err(jni_io)?,
            object: env.new_global_ref(object).map_err(jni_io)?,
            index: OnceLock::new(),
        })
    }
    fn index(&self) -> io::Result<&Vec<GameDataEntry>> {
        if let Some(index) = self.index.get() {
            return Ok(index);
        }
        let mut env = self.vm.attach_current_thread().map_err(jni_io)?;
        let result = env.call_method(
            self.object.as_obj(),
            "listJson",
            "()Ljava/lang/String;",
            &[],
        );
        if env.exception_check().map_err(jni_io)? {
            env.exception_describe().map_err(jni_io)?;
            env.exception_clear().map_err(jni_io)?;
            return Err(io::Error::other(
                "SAF directory discovery failed; see logcat for the provider error",
            ));
        }
        let json = result.map_err(jni_io)?.l().map_err(jni_io)?;
        let json: String = env.get_string(&JString::from(json)).map_err(jni_io)?.into();
        let index: Vec<DocumentEntry> = serde_json::from_str(&json).map_err(jni_io)?;
        let _ = self.index.set(
            index
                .into_iter()
                .map(|entry| GameDataEntry {
                    path: entry.path,
                    metadata: GameDataMetadata {
                        directory: entry.directory,
                        size: u64::try_from(entry.size).ok(),
                        modified: (entry.modified != 0).then_some(entry.modified),
                    },
                })
                .collect(),
        );
        Ok(self.index.get().unwrap())
    }
}
impl GameDataSource for AndroidSafDataSource {
    fn open(&self, path: &str) -> io::Result<Box<dyn GameDataReader>> {
        asset_transport::data_source::validate_relative_path(path)?;
        let mut env = self.vm.attach_current_thread().map_err(jni_io)?;
        let name = env.new_string(path).map_err(jni_io)?;
        let result = env.call_method(
            self.object.as_obj(),
            "openFd",
            "(Ljava/lang/String;)I",
            &[JValue::Object(name.as_ref())],
        );
        if env.exception_check().map_err(jni_io)? {
            env.exception_describe().map_err(jni_io)?;
            env.exception_clear().map_err(jni_io)?;
            return Err(io::Error::other(format!(
                "SAF could not open {path}; see logcat for the provider error"
            )));
        }
        let fd = result.map_err(jni_io)?.i().map_err(jni_io)?;
        if fd < 0 {
            return Err(io::Error::other("SAF returned an invalid descriptor"));
        }
        // openFd transfers a newly detached descriptor; this File is its sole owner.
        Ok(Box::new(unsafe { std::fs::File::from_raw_fd(fd) }))
    }
    fn metadata(&self, path: &str) -> io::Result<GameDataMetadata> {
        if path.is_empty() {
            return Ok(GameDataMetadata {
                directory: true,
                size: None,
                modified: None,
            });
        }
        self.index()?
            .iter()
            .find(|entry| entry.path == path)
            .map(|entry| entry.metadata.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.to_owned()))
    }
    fn read_dir(&self, path: &str) -> io::Result<Vec<GameDataEntry>> {
        asset_transport::data_source::validate_relative_path(path)?;
        Ok(self
            .index()?
            .iter()
            .filter(|entry| entry.path.rsplit_once('/').map_or("", |(parent, _)| parent) == path)
            .cloned()
            .collect())
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_nativecod_app_RuntimeActivity_nativeConfigure(
    mut env: JNIEnv,
    _object: JObject,
    root: JString,
    files: JString,
    diagnostic: jboolean,
    source: JObject,
) {
    let result = (|| -> io::Result<LaunchConfig> {
        let root: String = env.get_string(&root).map_err(jni_io)?.into();
        let files: String = env.get_string(&files).map_err(jni_io)?.into();
        let source = if source.is_null() {
            None
        } else {
            Some(Arc::new(AndroidSafDataSource::new(&mut env, source)?))
        };
        Ok(LaunchConfig {
            source,
            root: (!root.is_empty()).then(|| PathBuf::from(root)),
            files: files.into(),
            diagnostic: diagnostic != 0,
        })
    })();
    match result {
        Ok(config) => {
            if CONFIG.set(config).is_err() {
                let _ = env.throw_new(
                    "java/lang/IllegalStateException",
                    "Runtime already configured",
                );
            }
        }
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_nativecod_app_RuntimeActivity_nativeRequestExit(
    _env: JNIEnv,
    _object: JObject,
) {
    EXIT.store(true, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
struct PadState {
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    lt: f32,
    rt: f32,
    buttons: u32,
}

static PAD: Mutex<PadState> = Mutex::new(PadState {
    lx: 0.0,
    ly: 0.0,
    rx: 0.0,
    ry: 0.0,
    lt: 0.0,
    rt: 0.0,
    buttons: 0,
});

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_nativecod_app_RuntimeActivity_nativePad(
    _env: JNIEnv,
    _object: JObject,
    lx: jfloat,
    ly: jfloat,
    rx: jfloat,
    ry: jfloat,
    lt: jfloat,
    rt: jfloat,
    buttons: jint,
) {
    *PAD.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = PadState {
        lx,
        ly,
        rx,
        ry,
        lt,
        rt,
        buttons: buttons as u32,
    };
}

#[derive(Resource)]
struct Report(Arc<Mutex<String>>);

#[bevy_main]
pub fn main() {
    let config = CONFIG
        .get()
        .expect("GameActivity did not configure the native runtime");
    let report = Arc::new(Mutex::new(String::new()));
    #[cfg(feature = "iw4-runtime")]
    if !config.diagnostic {
        match start_match(config) {
            Ok(()) => return,
            Err(error) => {
                *report
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = error;
            }
        }
    }
    if !config.diagnostic && cfg!(not(feature = "iw4-runtime")) {
        let report = report.clone();
        let source = config.source.clone();
        std::thread::spawn(move || {
            let result = parse_installation(source);
            *report
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                result.unwrap_or_else(|error| format!("Game data load failed: {error}"));
        });
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        ..default()
                    })),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.06, 0.13, 0.09)))
        .insert_resource(bevy::winit::WinitSettings::mobile())
        .insert_resource(Report(report))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_report, lifecycle))
        .run();
}

fn parse_installation(source: Option<Arc<AndroidSafDataSource>>) -> Result<String, String> {
    let source = source.ok_or("No installation was selected")?;
    let files = asset_transport::data_source::find_game_files(source.as_ref())
        .map_err(|error| error.to_string())?;
    let common = files
        .iter()
        .find(|entry| {
            entry
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("common_mp.ff"))
        })
        .ok_or("common_mp.ff was not found")?;
    let mut header = [0; fastfile_iw4::FILE_PREAMBLE_LEN];
    source
        .open(&common.path)
        .map_err(|error| error.to_string())?
        .read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    fastfile_iw4::parse_file_header(&header).map_err(|error| format!("common_mp.ff: {error:?}"))?;
    let file = GameDataFile {
        source: source.clone(),
        path: common.path.clone(),
    };
    let zone = asset_transport::zone::open_zone_from(&file).map_err(|error| error.to_string())?;
    let table = zone.iw4_table().map_err(|error| error.to_string())?;
    let mut result = format!(
        "ARM64 IW4 parser loaded common_mp.ff\n{} decompressed bytes | {} assets | {:?} wire format",
        zone.bytes.len(),
        table.asset_count,
        table.format()
    );
    drop(zone);
    if let Some(map) = files.iter().find(|entry| {
        entry
            .path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("mp_boneyard.ff"))
    }) {
        let zone = asset_transport::zone::open_zone_from(&GameDataFile {
            source,
            path: map.path.clone(),
        })
        .map_err(|error| error.to_string())?;
        let table = zone.iw4_table().map_err(|error| error.to_string())?;
        result.push_str(&format!(
            "\nmp_boneyard.ff: {} bytes | {} assets",
            zone.bytes.len(),
            table.asset_count
        ));
    }
    result.push_str("\nMap rendering integration is still in progress.");
    info!("{result}");
    Ok(result)
}

fn setup(mut commands: Commands, adapter: Res<RenderAdapter>, info: Res<RenderAdapterInfo>) {
    commands.spawn(Camera2d);
    let config = CONFIG.get().unwrap();
    let report = format!(
        "Architecture={}\nBackend={:?}\nGPU={}\nFeatures={:?}\nLimits={:?}",
        std::env::consts::ARCH,
        info.backend,
        info.name,
        adapter.features(),
        adapter.limits()
    );
    info!("{report}");
    let _ = std::fs::write(config.files.join("gpu-report.txt"), &report);
    commands.spawn((
        Text::new(format!(
            "NATIVE COD | ARM64\nVulkan | {}\n{}\nBack to return to launcher",
            info.name,
            if config.diagnostic {
                "Native surface diagnostic"
            } else {
                "Reading MW2 data…"
            }
        )),
        TextFont {
            font_size: FontSize::Px(27.0),
            ..default()
        },
        TextColor(Color::srgb(0.78, 0.91, 0.49)),
        Node {
            position_type: PositionType::Absolute,
            top: px(48),
            left: px(48),
            right: px(48),
            ..default()
        },
    ));
}

fn update_report(
    report: Res<Report>,
    mut text: Query<&mut Text>,
    mut exit: MessageWriter<AppExit>,
    time: Res<Time>,
    mut clear: ResMut<ClearColor>,
    mut last: Local<String>,
) {
    let result = report
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    if !result.is_empty() && *last != result {
        if let Ok(mut text) = text.single_mut() {
            text.0 = format!("NATIVE COD | ARM64 / Vulkan\n{result}\nBack to return to launcher");
        }
        let _ = std::fs::write(
            CONFIG.get().unwrap().files.join("runtime-status.txt"),
            &result,
        );
        *last = result;
    }
    clear.0 = Color::srgb(0.06, 0.11 + 0.025 * time.elapsed_secs().sin(), 0.08);
    if EXIT.load(Ordering::Relaxed) {
        info!("Native runtime exiting cleanly");
        exit.write(AppExit::Success);
    }
}

fn lifecycle(mut messages: MessageReader<AppLifecycle>) {
    for event in messages.read() {
        info!("Android lifecycle: {event:?}");
    }
}

#[cfg(feature = "iw4-runtime")]
fn vm_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        let Some(rest) = line.strip_prefix("VmRSS:") else {
            continue;
        };
        return rest.split_whitespace().next()?.parse().ok();
    }
    None
}

#[cfg(feature = "iw4-runtime")]
fn apply_play_flags(files: &std::path::Path) {
    unsafe {
        std::env::set_var("IW4L_ANDROID_SLIM", "1");
        std::env::set_var("IW4L_SOUND", "off");
    }
    let path = files.join("play-flags.txt");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("play flags: slim defaults");
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.starts_with("IW4L_") {
            unsafe { std::env::set_var(key, value.trim()) };
        }
    }
    eprintln!("play flags: loaded {}", path.display());
}

#[cfg(feature = "iw4-runtime")]
fn start_match(config: &LaunchConfig) -> Result<(), String> {
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let Some(rss) = vm_rss_kb() else {
                continue;
            };
            static LAST: AtomicU64 = AtomicU64::new(0);
            let last = LAST.load(Ordering::Relaxed);
            if rss.saturating_sub(last) > 200_000 {
                eprintln!("rss {rss} KB");
                LAST.store(rss, Ordering::Relaxed);
            }
        }
    });
    apply_play_flags(&config.files);
    let root = config.root.clone().ok_or(
        "The selected folder is not on primary storage, so the match loader cannot open it.",
    )?;
    if std::fs::read_dir(&root).is_err() {
        return Err(format!(
            "Cannot read {}. Grant all-files access for NATIVE COD and press PLAY again.",
            root.display()
        ));
    }
    std::fs::canonicalize(&root)
        .map_err(|error| format!("Cannot resolve {}: {error}", root.display()))?;
    let files = &config.files;
    std::env::set_current_dir(files).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(files.join("iw4l-artifacts")).map_err(|error| error.to_string())?;
    unsafe {
        std::env::set_var("IW4L_GAMES", &root);
        std::env::set_var("IW4L_PIPELINED_RENDERING", "0");
    };
    if let Some(activity) = bevy::android::ANDROID_APP.get() {
        use bevy::android::android_activity::input::Axis;
        for axis in [
            Axis::X,
            Axis::Y,
            Axis::Z,
            Axis::Rx,
            Axis::Ry,
            Axis::Rz,
            Axis::HatX,
            Axis::HatY,
            Axis::Ltrigger,
            Axis::Rtrigger,
            Axis::Brake,
            Axis::Gas,
        ] {
            activity.enable_motion_axis(axis);
        }
    }
    let _ = bootstrap::ANDROID_SESSION.set(install_match_session);
    info!("starting IW4 menu from {}", root.display());
    bootstrap::launch(
        asset_transport::GamesRoot(root),
        files.join("iw4l-artifacts"),
        bootstrap::LaunchMode::Menu,
        None,
        Default::default(),
    );
    Ok(())
}

#[cfg(feature = "iw4-runtime")]
#[derive(Resource)]
struct RetroidPad(Entity);

#[cfg(feature = "iw4-runtime")]
fn install_match_session(app: &mut App) {
    app.add_systems(Startup, connect_pad)
        .add_systems(Update, note_frame)
        .add_systems(
            PreUpdate,
            (
                sync_pad.before(bevy::input::gamepad::gamepad_connection_system),
                watch_exit,
            ),
        );
}

fn note_frame(mut n: Local<u32>) {
    *n += 1;
    if *n <= 8 {
        eprintln!("frame {}", *n);
    }
}

#[cfg(feature = "iw4-runtime")]
fn connect_pad(world: &mut World) {
    use bevy::input::gamepad::{GamepadConnection, GamepadConnectionEvent};
    let id = world.spawn_empty().id();
    world.write_message(GamepadConnectionEvent::new(
        id,
        GamepadConnection::Connected {
            name: "Retroid".to_owned(),
            vendor_id: None,
            product_id: None,
        },
    ));
    world.insert_resource(RetroidPad(id));
}

#[cfg(feature = "iw4-runtime")]
fn sync_pad(
    pad: Option<Res<RetroidPad>>,
    mut raw: MessageWriter<bevy::input::gamepad::RawGamepadEvent>,
) {
    use bevy::input::gamepad::{
        GamepadAxis, GamepadButton, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
        RawGamepadEvent,
    };
    let Some(pad) = pad else {
        return;
    };
    let state = PAD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let id = pad.0;
    for (axis, value) in [
        (GamepadAxis::LeftStickX, state.lx),
        (GamepadAxis::LeftStickY, -state.ly),
        (GamepadAxis::RightStickX, state.rx),
        (GamepadAxis::RightStickY, -state.ry),
    ] {
        raw.write(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
            id, axis, value,
        )));
    }
    for (mask, button, analog) in [
        (1 << 0, GamepadButton::South, 0.0),
        (1 << 1, GamepadButton::East, 0.0),
        (1 << 2, GamepadButton::West, 0.0),
        (1 << 3, GamepadButton::North, 0.0),
        (1 << 4, GamepadButton::LeftTrigger, 0.0),
        (1 << 5, GamepadButton::RightTrigger, 0.0),
        (1 << 6, GamepadButton::LeftTrigger2, state.lt),
        (1 << 7, GamepadButton::RightTrigger2, state.rt),
        (1 << 8, GamepadButton::Select, 0.0),
        (1 << 9, GamepadButton::Start, 0.0),
        (1 << 10, GamepadButton::DPadUp, 0.0),
        (1 << 11, GamepadButton::DPadDown, 0.0),
        (1 << 12, GamepadButton::DPadLeft, 0.0),
        (1 << 13, GamepadButton::DPadRight, 0.0),
        (1 << 14, GamepadButton::LeftThumb, 0.0),
        (1 << 15, GamepadButton::RightThumb, 0.0),
    ] {
        let value = analog.max(if state.buttons & mask != 0 { 1.0 } else { 0.0 });
        raw.write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            id, button, value,
        )));
    }
}

#[cfg(feature = "iw4-runtime")]
fn watch_exit(mut exit: MessageWriter<AppExit>) {
    if EXIT.load(Ordering::Relaxed) {
        exit.write(AppExit::Success);
    }
}
