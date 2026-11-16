//! Importing from Firefox (and Firefox-based browsers) on Linux: saved
//! passwords and bookmarks, read straight from a profile folder.
//!
//! Passwords: `logins.json` holds each username and password encrypted
//! with the profile's key, which `key4.db` (SQLite) stores encrypted under
//! the Primary Password (empty unless you set one). OyNIx follows NSS:
//! PBES2 (PBKDF2-HMAC-SHA256 + AES-256-CBC) for current profiles, and the
//! older PBE-SHA1-3DES. A profile with a Primary Password can't be read;
//! Firefox's own CSV export works for those.
//!
//! The databases are copied before reading, so Firefox can stay open.

use base64::Engine;
use serde::Serialize;
use sha1::{Digest as _, Sha1};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub path: String,
    pub browser: String,
    pub default: bool,
    pub has_logins: bool,
    pub has_bookmarks: bool,
}

/// Firefox profiles on this computer (regular, Snap, Flatpak, LibreWolf).
pub fn profiles() -> Vec<Profile> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let roots = [
        ("Firefox", home.join(".mozilla/firefox")),
        ("Firefox (Snap)", home.join("snap/firefox/common/.mozilla/firefox")),
        ("Firefox (Flatpak)", home.join(".var/app/org.mozilla.firefox/.mozilla/firefox")),
        ("LibreWolf", home.join(".librewolf")),
        ("LibreWolf (Flatpak)", home.join(".var/app/io.gitlab.librewolf-community/.librewolf")),
    ];
    let mut out = vec![];
    for (browser, root) in roots {
        let Ok(ini) = std::fs::read_to_string(root.join("profiles.ini")) else { continue };
        let mut cur: Option<(String, String, bool, bool)> = None; // name, path, relative, default
        let push = |c: Option<(String, String, bool, bool)>, out: &mut Vec<Profile>| {
            if let Some((name, path, rel, def)) = c {
                if path.is_empty() {
                    return;
                }
                let p = if rel { root.join(&path) } else { PathBuf::from(&path) };
                if p.is_dir() {
                    out.push(Profile {
                        name: if name.is_empty() { path.clone() } else { name },
                        has_logins: p.join("logins.json").exists(),
                        has_bookmarks: p.join("places.sqlite").exists(),
                        path: p.to_string_lossy().into_owned(),
                        browser: browser.to_string(),
                        default: def,
                    });
                }
            }
        };
        for line in ini.lines().map(str::trim) {
            if line.starts_with('[') {
                push(cur.take(), &mut out);
                if line.to_ascii_lowercase().starts_with("[profile") {
                    cur = Some((String::new(), String::new(), true, false));
                }
            } else if let (Some(c), Some((k, v))) = (cur.as_mut(), line.split_once('=')) {
                match k.trim() {
                    "Name" => c.0 = v.trim().to_string(),
                    "Path" => c.1 = v.trim().to_string(),
                    "IsRelative" => c.2 = v.trim() != "0",
                    "Default" => c.3 = v.trim() == "1",
                    _ => {}
                }
            }
        }
        push(cur.take(), &mut out);
    }
    out.sort_by(|a, b| b.default.cmp(&a.default).then(b.has_logins.cmp(&a.has_logins)));
    out
}

// ── DER ───────────────────────────────────────────────────────────────

/// One TLV: (tag, contents, rest).
fn tlv(b: &[u8]) -> Result<(u8, &[u8], &[u8]), String> {
    let bad = || "unreadable key data".to_string();
    let tag = *b.first().ok_or_else(bad)?;
    let l0 = *b.get(1).ok_or_else(bad)? as usize;
    let (len, start) = if l0 < 0x80 {
        (l0, 2)
    } else {
        let n = l0 & 0x7f;
        if n == 0 || n > 4 {
            return Err(bad());
        }
        let mut len = 0usize;
        for i in 0..n {
            len = (len << 8) | *b.get(2 + i).ok_or_else(bad)? as usize;
        }
        (len, 2 + n)
    };
    let body = b.get(start..start + len).ok_or_else(bad)?;
    Ok((tag, body, &b[start + len..]))
}

/// The elements of a SEQUENCE.
fn seq(b: &[u8]) -> Result<Vec<(u8, &[u8])>, String> {
    let (tag, mut body, _) = tlv(b)?;
    if tag != 0x30 {
        return Err("unreadable key data".into());
    }
    let mut out = vec![];
    while !body.is_empty() {
        let (t, c, rest) = tlv(body)?;
        // Keep the whole element (with header) for nested sequences.
        let whole = &body[..body.len() - rest.len()];
        out.push((t, if t == 0x30 { whole } else { c }));
        body = rest;
    }
    Ok(out)
}

fn int(b: &[u8]) -> usize {
    b.iter().fold(0usize, |a, x| (a << 8) | *x as usize)
}

const OID_PBES2: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x05, 0x0d];
const OID_PBE_SHA1_3DES: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x0c, 0x05, 0x01, 0x03];
const OID_AES256_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x2a];
const OID_3DES_CBC: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x03, 0x07];

// ── Ciphers ───────────────────────────────────────────────────────────

fn unpad(mut v: Vec<u8>) -> Vec<u8> {
    if let Some(&n) = v.last() {
        let n = n as usize;
        if (1..=16).contains(&n) && n <= v.len() && v[v.len() - n..].iter().all(|&x| x as usize == n) {
            v.truncate(v.len() - n);
        }
    }
    v
}

fn aes256_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use cbc::cipher::{BlockDecryptMut, KeyIvInit};
    let mut buf = data.to_vec();
    let d = cbc::Decryptor::<aes::Aes256>::new_from_slices(key, iv).map_err(|_| "bad AES key")?;
    let n = d.decrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buf).map_err(|_| "AES decryption failed")?.len();
    buf.truncate(n);
    Ok(unpad(buf))
}

fn des3_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use cbc::cipher::{BlockDecryptMut, KeyIvInit};
    let mut buf = data.to_vec();
    let d = cbc::Decryptor::<des::TdesEde3>::new_from_slices(&key[..24.min(key.len())], iv).map_err(|_| "bad 3DES key")?;
    let n = d.decrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buf).map_err(|_| "3DES decryption failed")?.len();
    buf.truncate(n);
    Ok(unpad(buf))
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    let k = ring::hmac::Key::new(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key);
    ring::hmac::sign(&k, data).as_ref().to_vec()
}

/// Decrypt an NSS password-based blob (`key4.db` items) with the global
/// salt and the Primary Password (usually empty).
fn decrypt_pbe(der: &[u8], global_salt: &[u8], primary: &[u8]) -> Result<Vec<u8>, String> {
    let top = seq(der)?;
    let (algo, cipher) = (top.first().ok_or("unreadable key data")?.1, top.get(1).ok_or("unreadable key data")?.1);
    let a = seq(algo)?;
    let oid = a.first().ok_or("unreadable key data")?.1;
    if oid == OID_PBES2 {
        let params = seq(a.get(1).ok_or("unreadable key data")?.1)?;
        let kdf = seq(params.first().ok_or("unreadable key data")?.1)?;
        let kdf_params = seq(kdf.get(1).ok_or("unreadable key data")?.1)?;
        let salt = kdf_params.first().ok_or("unreadable key data")?.1;
        let iterations = int(kdf_params.get(1).ok_or("unreadable key data")?.1);
        let enc = seq(params.get(1).ok_or("unreadable key data")?.1)?;
        if enc.first().map(|x| x.1) != Some(OID_AES256_CBC) {
            return Err("unsupported key encryption".into());
        }
        let iv_tail = enc.get(1).ok_or("unreadable key data")?.1;
        let mut iv = vec![0x04, 0x0e];
        iv.extend_from_slice(iv_tail);
        let mut h = Sha1::new();
        h.update(global_salt);
        h.update(primary);
        let pw = h.finalize();
        let mut key = [0u8; 32];
        let it = std::num::NonZeroU32::new(iterations.clamp(1, 10_000_000) as u32).unwrap();
        ring::pbkdf2::derive(ring::pbkdf2::PBKDF2_HMAC_SHA256, it, salt, &pw, &mut key);
        aes256_cbc(&key, &iv, cipher)
    } else if oid == OID_PBE_SHA1_3DES {
        let params = seq(a.get(1).ok_or("unreadable key data")?.1)?;
        let entry_salt = params.first().ok_or("unreadable key data")?.1;
        let mut h = Sha1::new();
        h.update(global_salt);
        h.update(primary);
        let hp = h.finalize();
        let mut pes = entry_salt.to_vec();
        pes.resize(20.max(entry_salt.len()), 0);
        let mut h = Sha1::new();
        h.update(hp);
        h.update(entry_salt);
        let chp = h.finalize();
        let k1 = hmac_sha1(&chp, &[pes.as_slice(), entry_salt].concat());
        let tk = hmac_sha1(&chp, &pes);
        let k2 = hmac_sha1(&chp, &[tk.as_slice(), entry_salt].concat());
        let k = [k1, k2].concat();
        des3_cbc(&k[..24], &k[k.len() - 8..], cipher)
    } else {
        Err("unsupported key encryption".into())
    }
}

fn copy_db(src: &Path) -> Result<tempdir::Dir, String> {
    let d = tempdir::Dir::new()?;
    let name = src.file_name().ok_or("bad path")?;
    std::fs::copy(src, d.0.join(name)).map_err(|e| format!("{}: {e}", src.display()))?;
    for ext in ["-wal", "-shm"] {
        let side = PathBuf::from(format!("{}{ext}", src.display()));
        if side.exists() {
            let _ = std::fs::copy(&side, d.0.join(format!("{}{ext}", name.to_string_lossy())));
        }
    }
    Ok(d)
}

mod tempdir {
    pub struct Dir(pub std::path::PathBuf);
    impl Dir {
        pub fn new() -> Result<Self, String> {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("oynix-import-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
            Ok(Dir(p))
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

/// The profile's key for logins.json.
fn master_key(profile: &Path) -> Result<Vec<u8>, String> {
    let tmp = copy_db(&profile.join("key4.db"))?;
    let db = rusqlite::Connection::open_with_flags(tmp.0.join("key4.db"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let (salt, check): (Vec<u8>, Vec<u8>) = db
        .query_row("SELECT item1, item2 FROM metaData WHERE id = 'password'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|_| "key4.db has no password entry")?;
    let ok = decrypt_pbe(&check, &salt, b"").map(|v| v.starts_with(b"password-check")).unwrap_or(false);
    if !ok {
        return Err("This Firefox profile has a Primary Password, so its passwords can't be read here. In Firefox, open about:logins › ⋯ › Export Passwords, then import that CSV file.".into());
    }
    const CKA_ID: [u8; 16] = [0xf8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let mut st = db.prepare("SELECT a11, a102 FROM nssPrivate").map_err(|e| e.to_string())?;
    let rows = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))).map_err(|e| e.to_string())?;
    for (a11, a102) in rows.flatten() {
        if a102 == CKA_ID {
            let k = decrypt_pbe(&a11, &salt, b"")?;
            return Ok(k);
        }
    }
    Err("key4.db has no key for saved logins".into())
}

fn decrypt_field(key: &[u8], b64: &str) -> Result<String, String> {
    let der = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| "bad base64 in logins.json")?;
    let top = seq(&der)?;
    let algo = seq(top.get(1).ok_or("bad login entry")?.1)?;
    let cipher = top.get(2).ok_or("bad login entry")?.1;
    let oid = algo.first().ok_or("bad login entry")?.1;
    let iv = algo.get(1).ok_or("bad login entry")?.1;
    let plain = if oid == OID_3DES_CBC {
        des3_cbc(key, iv, cipher)?
    } else if oid == OID_AES256_CBC {
        aes256_cbc(&key[..32.min(key.len())], iv, cipher)?
    } else {
        return Err("unsupported login encryption".into());
    };
    String::from_utf8(plain).map_err(|_| "a login didn't decrypt to text".into())
}

/// (url, username, password) for every saved login in the profile.
pub fn logins(profile: &Path) -> Result<Vec<(String, String, String)>, String> {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(profile.join("logins.json")).map_err(|_| "This profile has no saved passwords (no logins.json).")?)
        .map_err(|_| "logins.json is unreadable")?;
    let key = master_key(profile)?;
    let mut out = vec![];
    for l in json["logins"].as_array().cloned().unwrap_or_default() {
        let url = l["hostname"].as_str().unwrap_or("").to_string();
        let (Some(u), Some(p)) = (l["encryptedUsername"].as_str(), l["encryptedPassword"].as_str()) else { continue };
        match (decrypt_field(&key, u), decrypt_field(&key, p)) {
            (Ok(u), Ok(p)) => out.push((url, u, p)),
            (Err(e), _) | (_, Err(e)) => eprintln!("oynix: skipped a Firefox login for {url}: {e}"),
        }
    }
    Ok(out)
}

/// (url, title, folder) for every bookmark. Toolbar bookmarks go to OyNIx's
/// favourites; the rest keep their folder name, or "Firefox" at the top.
pub fn bookmarks(profile: &Path) -> Result<Vec<(String, String, String)>, String> {
    let tmp = copy_db(&profile.join("places.sqlite"))?;
    let db = rusqlite::Connection::open_with_flags(tmp.0.join("places.sqlite"), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let mut st = db
        .prepare(
            "SELECT p.url, COALESCE(b.title, ''), COALESCE(f.title, ''), COALESCE(f.guid, '')
             FROM moz_bookmarks b JOIN moz_places p ON p.id = b.fk LEFT JOIN moz_bookmarks f ON f.id = b.parent
             WHERE b.type = 1 AND p.url NOT LIKE 'place:%' ORDER BY b.parent, b.position",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))
        .map_err(|e| e.to_string())?;
    Ok(rows
        .flatten()
        .map(|(url, title, folder, guid)| {
            let folder = match guid.as_str() {
                "toolbar_____" => "favorites".to_string(),
                "menu________" | "unfiled_____" | "mobile______" | "root________" => "Firefox".to_string(),
                _ if folder.is_empty() => "Firefox".to_string(),
                _ => folder,
            };
            (url, title, folder)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbc::cipher::{BlockEncryptMut, KeyIvInit};

    fn der(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut v = vec![tag];
        if body.len() < 0x80 {
            v.push(body.len() as u8);
        } else {
            v.extend([0x82, (body.len() >> 8) as u8, body.len() as u8]);
        }
        v.extend_from_slice(body);
        v
    }
    fn sq(parts: &[Vec<u8>]) -> Vec<u8> {
        der(0x30, &parts.concat())
    }
    fn aes_enc(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
        let mut buf = data.to_vec();
        let pad = 16 - buf.len() % 16;
        buf.extend(std::iter::repeat_n(pad as u8, pad));
        let n = buf.len();
        cbc::Encryptor::<aes::Aes256>::new_from_slices(key, iv).unwrap().encrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buf, n).unwrap();
        buf
    }
    /// What NSS writes for a PBES2-protected item.
    fn pbes2(global_salt: &[u8], plain: &[u8]) -> Vec<u8> {
        let salt = [7u8; 32];
        let iv14 = [9u8; 14];
        let mut h = Sha1::new();
        h.update(global_salt);
        let pw = h.finalize();
        let mut key = [0u8; 32];
        ring::pbkdf2::derive(ring::pbkdf2::PBKDF2_HMAC_SHA256, std::num::NonZeroU32::new(1000).unwrap(), &salt, &pw, &mut key);
        let iv = [&[0x04u8, 0x0e][..], &iv14].concat();
        let hmac_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x02, 0x09];
        let pbkdf2_oid = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x05, 0x0c];
        let kdf = sq(&[der(6, &pbkdf2_oid), sq(&[der(4, &salt), der(2, &[0x03, 0xe8]), der(2, &[32]), sq(&[der(6, &hmac_oid)])])]);
        let enc = sq(&[der(6, OID_AES256_CBC), der(4, &iv14)]);
        sq(&[sq(&[der(6, OID_PBES2), sq(&[kdf, enc])]), der(4, &aes_enc(&key, &iv, plain))])
    }

    #[test]
    fn reads_a_firefox_profile() {
        let dir = std::env::temp_dir().join(format!("oynix-ff-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let global_salt = [3u8; 20];
        let master = [5u8; 32];
        // key4.db
        let db = rusqlite::Connection::open(dir.join("key4.db")).unwrap();
        db.execute_batch("CREATE TABLE metaData (id PRIMARY KEY UNIQUE ON CONFLICT REPLACE, item1, item2); CREATE TABLE nssPrivate (id PRIMARY KEY, a11, a102);").unwrap();
        db.execute("INSERT INTO metaData VALUES ('password', ?1, ?2)", rusqlite::params![global_salt.to_vec(), pbes2(&global_salt, b"password-check\x02\x02")]).unwrap();
        let cka: Vec<u8> = [vec![0xf8], vec![0; 14], vec![1]].concat();
        db.execute("INSERT INTO nssPrivate VALUES (1, ?1, ?2)", rusqlite::params![pbes2(&global_salt, &master), cka]).unwrap();
        drop(db);
        // logins.json, fields encrypted with the master key (AES-256-CBC)
        let field = |s: &str| {
            let iv = [1u8; 16];
            let d = sq(&[der(4, &[0xf8; 16]), sq(&[der(6, OID_AES256_CBC), der(4, &iv)]), der(4, &aes_enc(&master, &iv, s.as_bytes()))]);
            base64::engine::general_purpose::STANDARD.encode(d)
        };
        let logins = serde_json::json!({ "logins": [{ "hostname": "https://site.example", "encryptedUsername": field("ana@example.com"), "encryptedPassword": field("hunter2 é") }] });
        std::fs::write(dir.join("logins.json"), logins.to_string()).unwrap();
        assert_eq!(super::logins(&dir).unwrap(), vec![("https://site.example".into(), "ana@example.com".into(), "hunter2 é".into())]);

        // places.sqlite
        let db = rusqlite::Connection::open(dir.join("places.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT);
             CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INT, fk INT, parent INT, position INT, title TEXT, guid TEXT);
             INSERT INTO moz_places VALUES (1, 'https://a.example/'), (2, 'https://b.example/'), (3, 'place:sort=8');
             INSERT INTO moz_bookmarks VALUES (10, 2, NULL, 0, 0, 'toolbar', 'toolbar_____'), (11, 2, NULL, 0, 1, 'menu', 'menu________'),
               (12, 2, NULL, 11, 0, 'Work', 'abcdefghijkl'), (20, 1, 1, 10, 0, 'A', 'x1'), (21, 1, 2, 12, 0, 'B', 'x2'), (22, 1, 3, 11, 1, 'Recent', 'x3');",
        )
        .unwrap();
        drop(db);
        let mut b = super::bookmarks(&dir).unwrap();
        b.sort();
        assert_eq!(b, vec![("https://a.example/".into(), "A".into(), "favorites".into()), ("https://b.example/".into(), "B".into(), "Work".into())]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn primary_password_is_reported() {
        let dir = std::env::temp_dir().join(format!("oynix-ff-pp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = rusqlite::Connection::open(dir.join("key4.db")).unwrap();
        db.execute_batch("CREATE TABLE metaData (id, item1, item2); CREATE TABLE nssPrivate (id, a11, a102);").unwrap();
        // "password-check" encrypted under a different (non-empty) password.
        db.execute("INSERT INTO metaData VALUES ('password', ?1, ?2)", rusqlite::params![vec![3u8; 20], pbes2(&[4u8; 20], b"password-check\x02\x02")]).unwrap();
        drop(db);
        std::fs::write(dir.join("logins.json"), r#"{"logins":[]}"#).unwrap();
        assert!(super::logins(&dir).unwrap_err().contains("Primary Password"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
