//! Saved passwords.
//!
//! Logins are kept in `logins.enc` in the profile's data folder, encrypted
//! with AES-256-GCM. The key is 32 random bytes in `logins.key` next to it,
//! readable only by you (0600) — the same protection Chromium gives
//! passwords on Linux without a keyring: it keeps them out of backups of
//! other files, sync folders and casual reading, not away from programs
//! running as you.
//!
//! Websites never see this store. OyNIx fills a form from a script world
//! the page can't reach, choosing logins by the tab's real address.

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{library::now, paths};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Login {
    pub id: String,
    /// scheme://host[:port]
    pub origin: String,
    pub username: String,
    pub password: String,
    pub created: i64,
    pub last_used: i64,
    pub times_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Store {
    pub logins: Vec<Login>,
    /// Origins where you chose "Never" for saving.
    pub never: Vec<String>,
}

fn dir() -> PathBuf {
    paths::data_dir()
}

fn key() -> Result<LessSafeKey, String> {
    let f = dir().join("logins.key");
    let bytes = match std::fs::read(&f) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            let mut b = vec![0u8; 32];
            SystemRandom::new().fill(&mut b).map_err(|_| "no random numbers")?;
            write_private(&f, &b)?;
            b
        }
    };
    UnboundKey::new(&AES_256_GCM, &bytes).map(LessSafeKey::new).map_err(|_| "bad key".to_string())
}

fn write_private(path: &std::path::Path, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp).map_err(|e| e.to_string())?;
    f.write_all(data).map_err(|e| e.to_string())?;
    f.sync_all().ok();
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn load() -> Store {
    let Ok(blob) = std::fs::read(dir().join("logins.enc")) else { return Store::default() };
    let Ok(k) = key() else { return Store::default() };
    if blob.len() < 12 + 16 {
        return Store::default();
    }
    let (nonce, rest) = blob.split_at(12);
    let mut buf = rest.to_vec();
    let Ok(nonce) = Nonce::try_assume_unique_for_key(nonce) else { return Store::default() };
    match k.open_in_place(nonce, Aad::empty(), &mut buf) {
        Ok(plain) => serde_json::from_slice(plain).unwrap_or_default(),
        Err(_) => {
            eprintln!("oynix: saved passwords couldn't be decrypted (logins.key changed?)");
            Store::default()
        }
    }
}

pub fn save(store: &Store) -> Result<(), String> {
    let k = key()?;
    let mut nonce = [0u8; 12];
    SystemRandom::new().fill(&mut nonce).map_err(|_| "no random numbers")?;
    let mut buf = serde_json::to_vec(store).map_err(|e| e.to_string())?;
    k.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut buf).map_err(|_| "encryption failed")?;
    let mut out = nonce.to_vec();
    out.extend(buf);
    write_private(&dir().join("logins.enc"), &out)
}

/// `https://accounts.example.com/login?x` -> `https://accounts.example.com`.
pub fn origin_of(url: &str) -> String {
    url::Url::parse(url).map(|u| u.origin().ascii_serialization()).unwrap_or_default()
}

fn new_id() -> String {
    let mut b = [0u8; 8];
    let _ = SystemRandom::new().fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Store {
    /// Logins for a page, most recently used first.
    pub fn for_url(&self, url: &str) -> Vec<&Login> {
        let o = origin_of(url);
        let mut v: Vec<&Login> = self.logins.iter().filter(|l| !o.is_empty() && l.origin == o).collect();
        v.sort_by(|a, b| b.last_used.cmp(&a.last_used).then(b.created.cmp(&a.created)));
        v
    }

    /// Save or update. Returns "saved", "updated" or "same".
    pub fn put(&mut self, url: &str, username: &str, password: &str) -> &'static str {
        let o = origin_of(url);
        if o.is_empty() || password.is_empty() {
            return "same";
        }
        if let Some(l) = self.logins.iter_mut().find(|l| l.origin == o && l.username == username) {
            if l.password == password {
                return "same";
            }
            l.password = password.to_string();
            l.last_used = now();
            return "updated";
        }
        self.logins.push(Login { id: new_id(), origin: o, username: username.into(), password: password.into(), created: now(), last_used: now(), times_used: 0 });
        "saved"
    }

    /// What saving this would do, without saving: "saved", "updated", "same".
    pub fn would(&self, url: &str, username: &str, password: &str) -> &'static str {
        let o = origin_of(url);
        match self.logins.iter().find(|l| l.origin == o && l.username == username) {
            Some(l) if l.password == password => "same",
            Some(_) => "updated",
            None if password.is_empty() => "same",
            None => "saved",
        }
    }

    pub fn used(&mut self, id: &str) {
        if let Some(l) = self.logins.iter_mut().find(|l| l.id == id) {
            l.last_used = now();
            l.times_used += 1;
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.logins.len();
        self.logins.retain(|l| l.id != id);
        n != self.logins.len()
    }

    /// Add logins from an import; skips ones already saved. Returns how many were new.
    pub fn merge(&mut self, imported: Vec<(String, String, String)>) -> usize {
        let mut n = 0;
        for (url, user, pass) in imported {
            let o = origin_of(&url);
            if o.is_empty() || pass.is_empty() || self.logins.iter().any(|l| l.origin == o && l.username == user && l.password == pass) {
                continue;
            }
            if let Some(l) = self.logins.iter_mut().find(|l| l.origin == o && l.username == user) {
                l.password = pass;
            } else {
                self.logins.push(Login { id: new_id(), origin: o, username: user, password: pass, created: now(), last_used: 0, times_used: 0 });
            }
            n += 1;
        }
        n
    }
}

/// Logins from a CSV export (Firefox, Chrome, Edge, Bitwarden, 1Password…):
/// any file with url/origin/login_uri, username/login_username and
/// password/login_password columns.
pub fn parse_csv(text: &str) -> Result<Vec<(String, String, String)>, String> {
    let rows = csv_rows(text.trim_start_matches('\u{feff}'));
    let header: Vec<String> = rows.first().ok_or("the file is empty")?.iter().map(|h| h.trim().to_ascii_lowercase()).collect();
    let col = |names: &[&str]| header.iter().position(|h| names.contains(&h.as_str()));
    let url = col(&["url", "origin", "login_uri", "website", "hostname", "origin_url"]).ok_or("no url column")?;
    let user = col(&["username", "login_username", "user", "login", "email"]).ok_or("no username column")?;
    let pass = col(&["password", "login_password"]).ok_or("no password column")?;
    Ok(rows
        .iter()
        .skip(1)
        .filter_map(|r| {
            let u = r.get(url)?.trim();
            let u = if u.contains("://") { u.to_string() } else { format!("https://{u}") };
            Some((u, r.get(user).cloned().unwrap_or_default(), r.get(pass)?.clone()))
        })
        .filter(|(u, _, p)| !p.is_empty() && !origin_of(u).is_empty())
        .collect())
}

/// RFC 4180 CSV: quoted fields with "" escapes and embedded newlines.
fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = vec![];
    let mut row = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_round_trip_and_rules() {
        std::env::set_var("XDG_DATA_HOME", std::env::temp_dir().join(format!("oynix-pw-{}", std::process::id())));
        let mut s = Store::default();
        assert_eq!(s.put("https://example.com/login?next=/", "ana", "pw1"), "saved");
        assert_eq!(s.put("https://example.com/other", "ana", "pw1"), "same");
        assert_eq!(s.would("https://example.com/", "ana", "pw2"), "updated");
        assert_eq!(s.put("https://example.com/", "ana", "pw2"), "updated");
        assert_eq!(s.put("http://example.com/", "ana", "pw3"), "saved", "http and https are different sites");
        assert_eq!(s.for_url("https://example.com/x").len(), 1);
        save(&s).unwrap();
        let raw = std::fs::read(dir().join("logins.enc")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("pw2"), "stored encrypted");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(dir().join("logins.key")).unwrap().permissions().mode() & 0o777, 0o600);
        let back = load();
        assert_eq!(back.logins, s.logins);
    }

    #[test]
    fn csv_formats() {
        // Firefox's export
        let ff = "\"url\",\"username\",\"password\",\"httpRealm\",\"formActionOrigin\",\"guid\",\"timeCreated\",\"timeLastUsed\",\"timePasswordChanged\"\n\"https://a.example\",\"ana\",\"p,\"\"q\",,\"https://a.example\",\"{1}\",\"1\",\"1\",\"1\"\n";
        assert_eq!(parse_csv(ff).unwrap(), vec![("https://a.example".into(), "ana".into(), "p,\"q".into())]);
        // Chrome's export
        let ch = "name,url,username,password,note\r\nb,https://b.example/login,bo,secret,\r\n";
        assert_eq!(parse_csv(ch).unwrap(), vec![("https://b.example/login".into(), "bo".into(), "secret".into())]);
        // Bitwarden's export
        let bw = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n,,login,C,,,0,c.example,cy,cpw,\n";
        assert_eq!(parse_csv(bw).unwrap(), vec![("https://c.example".into(), "cy".into(), "cpw".into())]);
        assert!(parse_csv("a,b\n1,2\n").is_err());
    }
}
