//! Graphics backend selection.
//!
//! The whole UI (and Chromium's compositor output) is drawn by Qt Quick's
//! scene graph through Qt's RHI. OyNIx prefers **Vulkan 1.3 / 1.2** and
//! falls back to **OpenGL ES 3** (via EGL), then desktop OpenGL, then the
//! software rasteriser. The Vulkan probe uses `ash` with the system loader
//! loaded at runtime, so the binary still starts on machines without Vulkan.

use ash::vk;
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Vulkan,
    Gles,
    OpenGl,
    Software,
}

impl Backend {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "vulkan" | "vk" => Some(Self::Vulkan),
            "gles" | "opengles" | "es" | "glfw-es" => Some(Self::Gles),
            "opengl" | "gl" => Some(Self::OpenGl),
            "software" | "sw" | "cpu" => Some(Self::Software),
            _ => None,
        }
    }

    pub fn id(self) -> i32 {
        match self {
            Self::Vulkan => 1,
            Self::Gles => 2,
            Self::OpenGl => 3,
            Self::Software => 4,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan",
            Self::Gles => "OpenGL ES",
            Self::OpenGl => "OpenGL",
            Self::Software => "Software",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VulkanInfo {
    pub api_major: u32,
    pub api_minor: u32,
    pub device: String,
    pub driver_is_cpu: bool,
}

impl VulkanInfo {
    pub fn version(&self) -> String {
        format!("{}.{}", self.api_major, self.api_minor)
    }
}

/// Probe for the best Vulkan device. Returns `None` if there is no loader,
/// no device, or no device supporting at least Vulkan 1.2.
pub fn probe_vulkan() -> Option<VulkanInfo> {
    // SAFETY: loading the system Vulkan loader; failure is handled.
    let entry = unsafe { ash::Entry::load() }.ok()?;
    let instance_version = unsafe { entry.try_enumerate_instance_version() }.ok().flatten().unwrap_or(vk::API_VERSION_1_0);
    if vk::api_version_minor(instance_version) < 2 && vk::api_version_major(instance_version) == 1 {
        return None;
    }
    let want = if instance_version >= vk::API_VERSION_1_3 { vk::API_VERSION_1_3 } else { vk::API_VERSION_1_2 };
    let app = vk::ApplicationInfo::default()
        .application_name(c"OyNIx")
        .engine_name(c"OyNIx-probe")
        .api_version(want);
    let ci = vk::InstanceCreateInfo::default().application_info(&app);
    let instance = unsafe { entry.create_instance(&ci, None) }.ok()?;
    let mut best: Option<(i32, VulkanInfo)> = None;
    if let Ok(devs) = unsafe { instance.enumerate_physical_devices() } {
        for d in devs {
            let p = unsafe { instance.get_physical_device_properties(d) };
            let api = p.api_version.min(want);
            let (maj, min) = (vk::api_version_major(api), vk::api_version_minor(api));
            if maj < 1 || (maj == 1 && min < 2) {
                continue;
            }
            let name = unsafe { CStr::from_ptr(p.device_name.as_ptr()) }.to_string_lossy().into_owned();
            let cpu = p.device_type == vk::PhysicalDeviceType::CPU;
            let rank = match p.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 4,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 3,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                vk::PhysicalDeviceType::CPU => 0,
                _ => 1,
            } * 10
                + min as i32;
            let info = VulkanInfo { api_major: maj, api_minor: min, device: name, driver_is_cpu: cpu };
            if best.as_ref().map(|(r, _)| rank > *r).unwrap_or(true) {
                best = Some((rank, info));
            }
        }
    }
    unsafe { instance.destroy_instance(None) };
    best.map(|(_, i)| i)
}

pub struct Selection {
    pub backend: Backend,
    pub detail: String,
}

/// Is this a Wayland session (where EGL / OpenGL ES is the native path)?
fn wayland_session() -> bool {
    let platform = std::env::var("QT_QPA_PLATFORM").unwrap_or_default();
    if platform.starts_with("xcb") {
        return false;
    }
    platform.starts_with("wayland") || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Pick a backend. An explicit request wins. On Wayland the default is
/// OpenGL ES (EGL): with Vulkan's FIFO presentation, a present waits for the
/// compositor's frame callback, which compositors such as Hyprland withhold
/// while the window isn't visible (another workspace, behind a fullscreen
/// window), so the render thread and then the whole UI froze. Qt's EGL path
/// waits for frame callbacks with a timeout instead. Elsewhere use Vulkan
/// when a real (non-CPU) 1.2+ device exists, else desktop OpenGL (GLX on
/// X11 cannot hand Chromium a GLES context).
pub fn select(requested: Option<Backend>) -> Selection {
    match requested {
        Some(Backend::Vulkan) => match probe_vulkan() {
            Some(v) => Selection { backend: Backend::Vulkan, detail: format!("Vulkan {} · {}", v.version(), v.device) },
            None => {
                eprintln!("oynix: Vulkan 1.2+ not available, falling back to OpenGL ES");
                Selection { backend: Backend::Gles, detail: "OpenGL ES (Vulkan unavailable)".into() }
            }
        },
        Some(b) => Selection { backend: b, detail: b.name().to_string() },
        None if wayland_session() => Selection { backend: Backend::Gles, detail: "OpenGL ES (EGL) · Wayland default".into() },
        None => match probe_vulkan() {
            Some(v) if !v.driver_is_cpu => Selection { backend: Backend::Vulkan, detail: format!("Vulkan {} · {}", v.version(), v.device) },
            _ => Selection { backend: Backend::OpenGl, detail: "OpenGL".into() },
        },
    }
}
