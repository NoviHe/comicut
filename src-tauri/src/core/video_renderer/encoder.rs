use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoder {
    Nvenc,
    Amf,
    X264,
}

impl Encoder {
    pub fn label(self) -> &'static str {
        match self {
            Encoder::Nvenc => "NVIDIA NVENC",
            Encoder::Amf => "AMD AMF",
            Encoder::X264 => "CPU x264",
        }
    }

    /// ffmpeg video-encoder arguments (quality-targeted, similar visual quality across all three).
    pub fn args(self) -> &'static [&'static str] {
        match self {
            Encoder::Nvenc => &["-c:v", "h264_nvenc", "-preset", "p5", "-rc", "vbr", "-cq", "23", "-b:v", "0"],
            Encoder::Amf => &["-c:v", "h264_amf", "-quality", "balanced", "-rc", "cqp", "-qp_i", "22", "-qp_p", "24"],
            Encoder::X264 => &["-c:v", "libx264", "-preset", "veryfast"],
        }
    }
}

pub struct GpuInfo {
    pub names: Vec<String>,
    pub encoder: Encoder,
}

fn quiet(program: &str) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Installed video adapters (Windows only; empty elsewhere, then both vendors are probed).
fn gpu_names() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    quiet("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "(Get-CimInstance Win32_VideoController).Name"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Real test encode at render settings (1080x1920 yuv420p 60fps): a listed encoder can still fail
/// to open when the driver is missing (e.g. "Cannot load nvcuda.dll").
fn works(enc: Encoder) -> bool {
    quiet("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=1080x1920:r=60:d=0.2", "-pix_fmt", "yuv420p"])
        .args(enc.args())
        .args(["-frames:v", "5", "-f", "null", "-"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn candidates(names: &[String]) -> Vec<Encoder> {
    let lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let nv = lower.iter().any(|n| n.contains("nvidia"));
    let amd = lower.iter().any(|n| n.contains("amd") || n.contains("radeon"));
    let mut out = Vec::new();
    if nv {
        out.push(Encoder::Nvenc);
    }
    if amd {
        out.push(Encoder::Amf);
    }
    if out.is_empty() {
        // Unknown / undetectable adapter: try both.
        out = vec![Encoder::Nvenc, Encoder::Amf];
    }
    out
}

/// Detected once per run. Order: GPU vendors present first, first one that really encodes wins, else CPU.
pub fn detect() -> &'static GpuInfo {
    static CACHE: OnceLock<GpuInfo> = OnceLock::new();
    CACHE.get_or_init(|| {
        let names = gpu_names();
        let encoder = candidates(&names).into_iter().find(|e| works(*e)).unwrap_or(Encoder::X264);
        println!("[gpu] adapters={:?} -> encoder {}", names, encoder.label());
        GpuInfo { names, encoder }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_order() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(candidates(&s(&["NVIDIA GeForce RTX 3060"])), vec![Encoder::Nvenc]);
        assert_eq!(candidates(&s(&["AMD Radeon RX 6600"])), vec![Encoder::Amf]);
        assert_eq!(candidates(&s(&["AMD Radeon(TM) Graphics", "NVIDIA GeForce GTX 1650"])), vec![Encoder::Nvenc, Encoder::Amf]);
        assert_eq!(candidates(&[]), vec![Encoder::Nvenc, Encoder::Amf]);
    }
}
