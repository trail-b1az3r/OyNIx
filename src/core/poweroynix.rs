//! `.poweroynix`: OyNIx extensions with full power.
//!
//! A `.poweroynix` holds the same files as an ordinary `.oynix` (`oynix.json`,
//! `code.ts`, `content.css`, optionally `graphics.qt`, `extra.html`,
//! `extra.jsonl`, `theme.json`), and may also carry a Chrome or Firefox
//! `manifest.json` (Manifest V2 or V3) whose background page, popup and
//! content scripts run alongside. In exchange for that power it has to be
//! verified: `oynix.sig` must be an Ed25519 signature, by one of the keys in
//! `packaging/poweroynix/trusted-keys.txt` (compiled in), over every other
//! file in the package. Without it OyNIx refuses to install the package, and
//! the website refuses to list it. Extensions that ship inside OyNIx are the
//! one exception: they come with the browser, so they are trusted already.
//!
//! Verified power extensions may use:
//! * the powerful WebExtension APIs (native messaging, webRequest, cookies,
//!   browsingData, management) without Developer mode, as long as their
//!   manifest asks for the permission;
//! * `"permissions": ["filters"]` in oynix.json: `filters.txt` (Adblock /
//!   uBlock syntax) is added to OyNIx's content blocker while the extension
//!   is on.
//!
//! The signed message is a header line followed by one line per file,
//! sorted by path: `<hash hex>  <path>\n` (paths relative, `/`-separated).
//! Format 2 (`"format": 2` in oynix.sig, the default since 5.3.1) hashes
//! files with SHA3-256; format 1 (SHA-256) is still accepted.
//! `scripts/poweroynix.py` makes keys, signs and verifies with the same rules.

use std::path::Path;

pub const MAGIC: &str = "OyNIx poweroynix v1\n";
pub const MAGIC_V2: &str = "OyNIx poweroynix v2\n";
pub const SIG_FILE: &str = "oynix.sig";
const TRUSTED_KEYS: &str = include_str!("../../packaging/poweroynix/trusted-keys.txt");
/// Packages bigger than this are refused rather than hashed.
const MAX_BYTES: u64 = 64 << 20;

/// Abilities only verified power extensions get, as oynix.json permissions.
pub const POWER_PERMISSIONS: &[&str] = &["filters"];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// (public key, holder) pairs from trusted-keys.txt (or the given text).
fn parse_keys(text: &str) -> Vec<(Vec<u8>, String)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (k, who) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
            unhex(k).filter(|k| k.len() == 32).map(|k| (k, who.trim().to_string()))
        })
        .collect()
}

/// Does this unpacked package say it's a power package?
pub fn is_power_dir(dir: &Path) -> bool {
    if dir.join(SIG_FILE).exists() {
        return true;
    }
    std::fs::read_to_string(dir.join("oynix.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).ok())
        .is_some_and(|v| v.get("power").and_then(|p| p.as_bool()) == Some(true))
}

/// The message that gets signed for the package in `dir` (format 2, SHA3-256).
#[cfg(test)]
pub fn digest_message(dir: &Path) -> Result<String, String> {
    digest_message_format(dir, 2)
}

/// The signed message in a given format: 1 = SHA-256, 2 = SHA3-256.
pub fn digest_message_format(dir: &Path, format: u64) -> Result<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, std::path::PathBuf)>, total: &mut u64) -> Result<(), String> {
        let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for e in rd.flatten() {
            let ty = e.file_type().map_err(|e| e.to_string())?;
            let p = e.path();
            if ty.is_symlink() {
                return Err(format!("{} is a symbolic link; packages can't contain links", p.display()));
            }
            if ty.is_dir() {
                walk(base, &p, out, total)?;
            } else if ty.is_file() {
                let rel = p.strip_prefix(base).map_err(|e| e.to_string())?;
                let rel: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                let rel = rel.join("/");
                if rel == SIG_FILE {
                    continue;
                }
                *total += e.metadata().map(|m| m.len()).unwrap_or(0);
                if *total > MAX_BYTES {
                    return Err("the package is larger than 64 MB".into());
                }
                out.push((rel, p));
            }
        }
        Ok(())
    }
    let mut files = vec![];
    walk(dir, dir, &mut files, &mut 0)?;
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut msg = String::from(if format >= 2 { MAGIC_V2 } else { MAGIC });
    for (rel, p) in files {
        if rel.contains('\n') {
            return Err("a file name contains a line break".into());
        }
        let bytes = std::fs::read(&p).map_err(|e| format!("{rel}: {e}"))?;
        let digest = if format >= 2 {
            use sha3::Digest;
            hex(&sha3::Sha3_256::digest(&bytes))
        } else {
            use sha2::Digest;
            hex(&sha2::Sha256::digest(&bytes))
        };
        msg.push_str(&digest);
        msg.push_str("  ");
        msg.push_str(&rel);
        msg.push('\n');
    }
    Ok(msg)
}

/// Check `dir`'s `oynix.sig` against the trusted keys. Returns who verified it.
pub fn verify(dir: &Path) -> Result<String, String> {
    verify_with(dir, TRUSTED_KEYS)
}

fn verify_with(dir: &Path, keys_text: &str) -> Result<String, String> {
    const UNVERIFIED: &str = "This is a .poweroynix extension, which can use OyNIx's most powerful APIs, so it has to be verified before it can be installed";
    let sig_text = std::fs::read_to_string(dir.join(SIG_FILE)).map_err(|_| format!("{UNVERIFIED}, and it has no signature (oynix.sig)."))?;
    let sig: serde_json::Value = serde_json::from_str(&sig_text).map_err(|e| format!("{UNVERIFIED}; its oynix.sig is unreadable: {e}"))?;
    let key = sig["key"].as_str().and_then(unhex).filter(|k| k.len() == 32).ok_or(format!("{UNVERIFIED}; oynix.sig has no valid \"key\"."))?;
    let signature = sig["signature"].as_str().and_then(unhex).filter(|s| s.len() == 64).ok_or(format!("{UNVERIFIED}; oynix.sig has no valid \"signature\"."))?;
    let keys = parse_keys(keys_text);
    let Some((_, who)) = keys.iter().find(|(k, _)| *k == key) else {
        return Err(format!("{UNVERIFIED}, and the key that signed it ({}…) isn't one OyNIx trusts.", &hex(&key)[..16]));
    };
    let format = sig["format"].as_u64().unwrap_or(1);
    if format > 2 {
        return Err(format!("{UNVERIFIED}; its oynix.sig uses format {format}, which needs a newer OyNIx."));
    }
    let msg = digest_message_format(dir, format)?;
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, &key)
        .verify(msg.as_bytes(), &signature)
        .map_err(|_| format!("{UNVERIFIED}. Its signature doesn't match its files: it was changed after it was verified."))?;
    Ok(if who.is_empty() { hex(&key)[..16].to_string() } else { who.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;

    fn pkg(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("oynix-power-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("icons")).unwrap();
        std::fs::write(d.join("oynix.json"), r#"{"name": "P", "power": true}"#).unwrap();
        std::fs::write(d.join("code.ts"), "console.log(1)").unwrap();
        std::fs::write(d.join("icons/a.png"), [0u8, 1, 2]).unwrap();
        d
    }

    fn sign(dir: &Path) -> String {
        let rng = ring::rand::SystemRandom::new();
        let doc = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let kp = ring::signature::Ed25519KeyPair::from_pkcs8(doc.as_ref()).unwrap();
        let sig = kp.sign(digest_message(dir).unwrap().as_bytes());
        let pubhex = hex(kp.public_key().as_ref());
        std::fs::write(dir.join(SIG_FILE), serde_json::json!({ "format": 2, "key": pubhex, "signature": hex(sig.as_ref()) }).to_string()).unwrap();
        pubhex
    }

    #[test]
    fn digest_is_sorted_and_skips_signature() {
        let d = pkg("digest");
        std::fs::write(d.join(SIG_FILE), "{}").unwrap();
        let m = digest_message(&d).unwrap();
        let paths: Vec<&str> = m.lines().skip(1).map(|l| l.split("  ").nth(1).unwrap()).collect();
        assert_eq!(paths, ["code.ts", "icons/a.png", "oynix.json"]);
        assert!(m.starts_with(MAGIC_V2));
        // SHA3-256 of "console.log(1)"
        use sha3::Digest;
        let want: String = sha3::Sha3_256::digest(b"console.log(1)").iter().map(|b| format!("{b:02x}")).collect();
        assert!(m.contains(&format!("{want}  code.ts")));
        assert!(digest_message_format(&d, 1).unwrap().starts_with(MAGIC));
    }

    #[test]
    fn verifies_trusted_signatures_only() {
        let d = pkg("verify");
        assert!(is_power_dir(&d));
        assert!(verify_with(&d, "").unwrap_err().contains("no signature"));
        let key = sign(&d);
        assert!(verify_with(&d, "").unwrap_err().contains("isn't one OyNIx trusts"));
        assert_eq!(verify_with(&d, &format!("# c\n{key}  Test Maintainer\n")).unwrap(), "Test Maintainer");
        // Any change after signing breaks it.
        std::fs::write(d.join("code.ts"), "console.log(2)").unwrap();
        assert!(verify_with(&d, &format!("{key} x")).unwrap_err().contains("changed after"));
        std::fs::write(d.join("code.ts"), "console.log(1)").unwrap();
        std::fs::write(d.join("extra.js"), "evil()").unwrap();
        assert!(verify_with(&d, &format!("{key} x")).unwrap_err().contains("changed after"));
    }

    #[test]
    fn format_1_signatures_still_verify() {
        let d = pkg("v1");
        let rng = ring::rand::SystemRandom::new();
        let doc = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let kp = ring::signature::Ed25519KeyPair::from_pkcs8(doc.as_ref()).unwrap();
        let sig = kp.sign(digest_message_format(&d, 1).unwrap().as_bytes());
        let key = hex(kp.public_key().as_ref());
        std::fs::write(d.join(SIG_FILE), serde_json::json!({ "key": key, "signature": hex(sig.as_ref()) }).to_string()).unwrap();
        assert_eq!(verify_with(&d, &format!("{key} Old")).unwrap(), "Old");
        // ...but not if the file claims format 2.
        std::fs::write(d.join(SIG_FILE), serde_json::json!({ "format": 2, "key": key, "signature": hex(sig.as_ref()) }).to_string()).unwrap();
        assert!(verify_with(&d, &format!("{key} Old")).is_err());
    }

    #[test]
    fn python_signer_agrees() {
        // A signature made by scripts/poweroynix.py (pure-Python Ed25519)
        // over the same files must verify here.
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/poweroynix.py");
        let d = pkg("python");
        let keyfile = d.with_extension("key");
        let run = |args: &[&str]| std::process::Command::new("python3").arg("-I").arg(&script).args(args).output();
        let Ok(out) = run(&["keygen", "Py Test", "--out", keyfile.to_str().unwrap()]) else { return };
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let public = String::from_utf8_lossy(&out.stdout).lines().find(|l| l.len() > 64 && l.as_bytes()[64] == b' ').unwrap().to_string();
        let out = run(&["sign", d.to_str().unwrap(), "--key", keyfile.to_str().unwrap()]).unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(verify_with(&d, &public).unwrap(), "Py Test");
        let _ = std::fs::remove_file(keyfile);
    }
}
