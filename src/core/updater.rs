//! Built-in updater.
//!
//! * **Check**: ask GitHub's release API for the newest release (stable, or
//!   including pre-releases on the beta channel). Nothing identifying is
//!   sent beyond what any HTTPS request carries.
//! * **Download**: the package that matches how this copy was installed
//!   (`.deb` for dpkg, `.pkg.tar.zst` for pacman), streamed to the cache,
//!   then checked against the release's `SHA256SUMS`.
//! * **Install**: through the system package manager, behind the desktop's
//!   password prompt (`pkexec`). OyNIx never runs as root itself.
//!
//! Copies that weren't installed from a package (built from source, or
//! `scripts/install.sh`) are told about new versions but not changed.
//!
//! `OYNIX_UPDATE_URL` overrides the release API (forks, testing) and
//! `OYNIX_UPDATE_DRY_RUN=1` prints the install command instead of running it.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use super::paths;

pub const RELEASES_API: &str = "https://api.github.com/repos/trail-b1az3r/OyNIx/releases?per_page=30";

// ── Versions ─────────────────────────────────────────────────────────

/// major.minor.patch plus an optional pre-release tag ("5.2.0-beta.1").
/// Parsing is lenient: older OyNIx tags look like "v3.2.0." or "v 4.1.1".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub nums: [u64; 3],
    pub pre: String,
}

impl Version {
    pub fn parse(s: &str) -> Option<Version> {
        let t = s.trim().trim_start_matches(|c: char| c == 'v' || c == 'V' || c.is_whitespace());
        let end = t.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(t.len());
        let (core, rest) = t.split_at(end);
        let parts: Vec<u64> = core.split('.').filter(|p| !p.is_empty()).map(|p| p.parse().ok()).collect::<Option<_>>()?;
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let mut nums = [0; 3];
        nums[..parts.len()].copy_from_slice(&parts);
        let pre = rest.trim_start_matches(['-', '+']).split('+').next().unwrap_or("").to_string();
        Some(Version { nums, pre })
    }

    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.nums[0], self.nums[1], self.nums[2])?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre)?;
        }
        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        self.nums.cmp(&o.nums).then_with(|| match (self.pre.is_empty(), o.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater, // 5.2.0 > 5.2.0-beta
            (false, true) => Ordering::Less,
            (false, false) => cmp_pre(&self.pre, &o.pre),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Compare dot-separated pre-release identifiers, numbers numerically.
fn cmp_pre(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.split('.'), b.split('.'));
    loop {
        match (x.next(), y.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) => {
                let o = match (p.parse::<u64>(), q.parse::<u64>()) {
                    (Ok(m), Ok(n)) => m.cmp(&n),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    _ => p.cmp(q),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
    }
}

pub fn current() -> Version {
    Version::parse(crate::VERSION).unwrap_or(Version { nums: [0, 0, 0], pre: String::new() })
}

// ── How this copy was installed ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Deb,
    Pacman,
    /// Built from source, `install.sh`, or anything else OyNIx can't update.
    Manual,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Deb => "deb",
            Kind::Pacman => "pacman",
            Kind::Manual => "manual",
        }
    }
}

pub fn install_kind() -> Kind {
    match std::env::var("OYNIX_UPDATE_KIND").as_deref() {
        Ok("deb") => return Kind::Deb,
        Ok("pacman") => return Kind::Pacman,
        Ok("manual") => return Kind::Manual,
        _ => {}
    }
    let exe = std::env::current_exe().ok().and_then(|p| p.canonicalize().ok()).unwrap_or_default();
    if !exe.starts_with("/usr/lib/oynix") && !exe.starts_with("/usr/bin") {
        return Kind::Manual;
    }
    if Path::new("/var/lib/dpkg/info/oynix.list").exists() {
        return Kind::Deb;
    }
    let pacman_owned = std::fs::read_dir("/var/lib/pacman/local")
        .map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("oynix-") && !e.file_name().to_string_lossy().starts_with("oynix-debug")))
        .unwrap_or(false);
    if pacman_owned {
        return Kind::Pacman;
    }
    Kind::Manual
}

// ── Releases ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    pub title: String,
    pub notes: String,
    pub page: String,
    pub published: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// The package for this kind of install, if the release has one.
    pub fn package_for(&self, kind: Kind) -> Option<&Asset> {
        let is_debug = |n: &str| n.contains("-debug") || n.contains("dbgsym");
        self.assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            !is_debug(&n)
                && match kind {
                    Kind::Deb => n.ends_with(".deb") && (n.contains("amd64") || n.contains("x86_64")),
                    Kind::Pacman => n.ends_with(".pkg.tar.zst") && n.contains("x86_64"),
                    Kind::Manual => false,
                }
        })
    }

    pub fn checksums(&self) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == "SHA256SUMS")
    }

    pub fn to_json(&self, kind: Kind) -> Value {
        json!({
            "version": self.version.to_string(),
            "title": self.title,
            "notes": self.notes,
            "page": self.page,
            "published": self.published,
            "package": self.package_for(kind).map(|a| json!({ "name": a.name, "size": a.size })),
        })
    }
}

/// Pick the newest usable release from the API's JSON.
pub fn pick_latest(releases: &Value, include_prerelease: bool) -> Option<Release> {
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str().unwrap_or("");
            let version = Version::parse(tag)?;
            let pre = r["prerelease"].as_bool().unwrap_or(false) || version.is_prerelease();
            if pre && !include_prerelease {
                return None;
            }
            let assets = r["assets"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| Asset {
                            name: x["name"].as_str().unwrap_or("").to_string(),
                            url: x["browser_download_url"].as_str().unwrap_or("").to_string(),
                            size: x["size"].as_u64().unwrap_or(0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Release {
                version,
                title: r["name"].as_str().unwrap_or(tag).to_string(),
                notes: r["body"].as_str().unwrap_or("").to_string(),
                page: r["html_url"].as_str().unwrap_or("").to_string(),
                published: r["published_at"].as_str().unwrap_or("").to_string(),
                assets,
            })
        })
        .max_by(|a, b| a.version.cmp(&b.version))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .user_agent(format!("OyNIx/{} (updater)", crate::VERSION))
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .build()
        .into()
}

/// Ask GitHub for the newest release. Ok(None): nothing newer than this copy.
pub fn check(include_prerelease: bool) -> Result<Option<Release>, String> {
    let url = std::env::var("OYNIX_UPDATE_URL").unwrap_or_else(|_| RELEASES_API.to_string());
    let mut resp = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Couldn’t reach the update server: {e}"))?;
    let text = resp.body_mut().with_config().limit(8 << 20).read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|_| "The update server sent something unexpected".to_string())?;
    Ok(pick_latest(&v, include_prerelease).filter(|r| r.version > current()))
}

// ── Download and verify ──────────────────────────────────────────────

pub fn download_dir() -> PathBuf {
    paths::cache_dir().join("updates")
}

/// Find `name`'s hash in a SHA256SUMS file ("<hex>  <name>" lines).
pub fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let hash = it.next()?;
        let file = it.next()?.trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
    })
}

/// Download the package for `kind` and check its SHA-256. `progress` gets
/// (received, total). Returns the verified file.
pub fn download(release: &Release, kind: Kind, cancel: &AtomicBool, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, String> {
    let asset = release.package_for(kind).ok_or("This release has no package for your system")?;
    let sums_asset = release.checksums().ok_or("This release has no SHA256SUMS file, so its download can’t be checked")?;
    let sums = agent()
        .get(&sums_asset.url)
        .call()
        .map_err(|e| format!("Couldn’t download the checksums: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let want = expected_hash(&sums, &asset.name).ok_or_else(|| format!("SHA256SUMS doesn’t list {}", asset.name))?;

    let dir = download_dir();
    let _ = std::fs::remove_dir_all(&dir); // one update at a time; drop old ones
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // The asset name comes from the server: keep only a plain file name.
    let safe: String = asset.name.chars().filter(|c| c.is_ascii_alphanumeric() || "._-+".contains(*c)).collect();
    let dest = dir.join(if safe.is_empty() || safe.starts_with('.') { "update.pkg".to_string() } else { safe });
    let part = dest.with_extension("part");

    let mut resp = agent().get(&asset.url).call().map_err(|e| format!("Download failed: {e}"))?;
    let total = resp.body().content_length().unwrap_or(asset.size);
    let mut reader = resp.body_mut().with_config().limit(2 << 30).reader();
    let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut got = 0u64;
    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            let _ = std::fs::remove_file(&part);
            return Err("Cancelled".into());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("Download failed: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| format!("Couldn’t save the update: {e}"))?;
        hasher.update(&buf[..n]);
        got += n as u64;
        progress(got, total);
    }
    file.flush().map_err(|e| e.to_string())?;
    let have: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if have != want {
        let _ = std::fs::remove_file(&part);
        return Err("The download didn’t match its published checksum and was deleted. Try again later.".into());
    }
    std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

// ── Install ──────────────────────────────────────────────────────────

/// The command that installs `pkg` as root.
pub fn install_command(kind: Kind, pkg: &Path) -> Option<Vec<String>> {
    let p = pkg.to_string_lossy().to_string();
    let mut cmd = vec!["pkexec".to_string()];
    match kind {
        // apt (not dpkg -i) so new dependencies are pulled in too.
        Kind::Deb => cmd.extend(["apt-get", "install", "-y", "--allow-downgrades"].map(String::from).into_iter().chain([p])),
        Kind::Pacman => cmd.extend(["pacman", "-U", "--noconfirm"].map(String::from).into_iter().chain([p])),
        Kind::Manual => return None,
    }
    Some(cmd)
}

/// What to type in a terminal instead (shown when pkexec is unavailable).
pub fn manual_command(kind: Kind, pkg: &Path) -> String {
    install_command(kind, pkg).map(|c| format!("sudo {}", c[1..].join(" "))).unwrap_or_default()
}

pub fn have_pkexec() -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("pkexec").is_file()))
}

/// Run the install. Blocks until the package manager finishes.
pub fn install(kind: Kind, pkg: &Path) -> Result<(), String> {
    let cmd = install_command(kind, pkg).ok_or("This copy of OyNIx wasn’t installed from a package")?;
    if std::env::var("OYNIX_UPDATE_DRY_RUN").is_ok_and(|v| v == "1") {
        eprintln!("oynix: update dry run: {}", cmd.join(" "));
        return Ok(());
    }
    if !have_pkexec() {
        return Err(format!("OyNIx needs pkexec to ask for your password. Install it, or run:\n{}", manual_command(kind, pkg)));
    }
    let out = std::process::Command::new(&cmd[0]).args(&cmd[1..]).output().map_err(|e| e.to_string())?;
    match out.status.code() {
        Some(0) => Ok(()),
        // pkexec: 126 = the user dismissed the password prompt, 127 = not authorised.
        Some(126) => Err("The update wasn’t installed because the password prompt was cancelled.".into()),
        // 127: not authorised, which is also what happens when no polkit
        // agent is running to show the prompt (common on minimal Wayland
        // setups such as a bare Hyprland: start hyprpolkitagent).
        Some(127) => Err("Your system didn’t show a password prompt. If you use Hyprland or another minimal desktop, start a polkit agent (for example hyprpolkitagent), or install the update from a terminal.".into()),
        _ => {
            let err = String::from_utf8_lossy(&out.stderr);
            let last = err.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("unknown error");
            Err(format!("The package manager couldn’t install the update: {last}"))
        }
    }
}

// ── Check schedule ───────────────────────────────────────────────────

fn stamp_file() -> PathBuf {
    paths::data_dir().join("update-check")
}

/// Seconds since the last successful check (u64::MAX if never).
pub fn since_last_check() -> u64 {
    let last: u64 = std::fs::read_to_string(stamp_file()).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if last == 0 {
        u64::MAX
    } else {
        now.saturating_sub(last)
    }
}

pub fn mark_checked() {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let _ = std::fs::write(stamp_file(), now.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        let v = |s| Version::parse(s).unwrap();
        assert_eq!(v("v5.1.0"), Version { nums: [5, 1, 0], pre: String::new() });
        assert_eq!(v("v3.2.0.").nums, [3, 2, 0]);
        assert_eq!(v("v 4.1.1").nums, [4, 1, 1]);
        assert_eq!(v("v1.1-dev").pre, "dev");
        assert_eq!(v("5.2").nums, [5, 2, 0]);
        assert!(Version::parse("latest").is_none());
        assert!(v("5.2.0") > v("5.1.9"));
        assert!(v("5.2.0") > v("5.2.0-beta.2"));
        assert!(v("5.2.0-beta.10") > v("5.2.0-beta.2"));
        assert!(v("5.2.0-rc.1") > v("5.2.0-beta.9"));
        assert!(v("5.10.0") > v("5.9.0"));
        assert_eq!(v("5.2.0-beta.1").to_string(), "5.2.0-beta.1");
    }

    fn releases() -> Value {
        json!([
            { "tag_name": "v5.3.0-beta.1", "prerelease": true, "draft": false, "name": "OyNIx 5.3.0 beta", "assets": [] },
            { "tag_name": "v9.9.9", "prerelease": false, "draft": true, "assets": [] },
            { "tag_name": "v5.2.0", "prerelease": false, "draft": false, "name": "OyNIx 5.2.0", "body": "Notes", "html_url": "https://x/r",
              "assets": [
                { "name": "oynix-debug-5.2.0-1-x86_64.pkg.tar.zst", "browser_download_url": "https://x/d", "size": 1 },
                { "name": "oynix-5.2.0-1-x86_64.pkg.tar.zst", "browser_download_url": "https://x/p", "size": 2 },
                { "name": "oynix_5.2.0_amd64.deb", "browser_download_url": "https://x/deb", "size": 3 },
                { "name": "SHA256SUMS", "browser_download_url": "https://x/s", "size": 4 } ] },
            { "tag_name": "v3.2.0.", "prerelease": false, "draft": false, "assets": [] }
        ])
    }

    #[test]
    fn picks_release_and_package() {
        let r = pick_latest(&releases(), false).unwrap();
        assert_eq!(r.version.to_string(), "5.2.0");
        assert_eq!(r.package_for(Kind::Pacman).unwrap().url, "https://x/p");
        assert_eq!(r.package_for(Kind::Deb).unwrap().url, "https://x/deb");
        assert!(r.package_for(Kind::Manual).is_none());
        assert_eq!(pick_latest(&releases(), true).unwrap().version.to_string(), "5.3.0-beta.1");
    }

    #[test]
    fn checksums_and_commands() {
        let sums = "aa  other\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789ABCDEF *oynix_5.2.0_amd64.deb\n";
        assert_eq!(expected_hash(sums, "oynix_5.2.0_amd64.deb").unwrap(), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        assert!(expected_hash(sums, "missing").is_none());
        assert!(expected_hash("short  x\n", "x").is_none());
        let p = Path::new("/tmp/u/oynix.deb");
        assert_eq!(install_command(Kind::Deb, p).unwrap().join(" "), "pkexec apt-get install -y --allow-downgrades /tmp/u/oynix.deb");
        assert_eq!(manual_command(Kind::Pacman, p), "sudo pacman -U --noconfirm /tmp/u/oynix.deb");
        assert!(install_command(Kind::Manual, p).is_none());
    }
}
