#!/usr/bin/env python3
"""Make keys for, sign, verify and pack .poweroynix extensions.

    poweroynix.py keygen "Your Name" [--out FILE]   new Ed25519 key pair
    poweroynix.py sign DIR --key FILE [--commit C] [--format N] [--pack OUT]
                                                    write DIR/oynix.sig (and a .poweroynix)
    poweroynix.py verify PATH [--keys FILE]         check a folder or .poweroynix
    poweroynix.py pack DIR OUT                      zip a signed folder

Signing is the "verified" step: read the extension first. The rules match
crates/stonedgem-poweroynix. The Ed25519 code is RFC 8032's reference
implementation, so this needs nothing beyond Python 3.
"""

import argparse
import hashlib
import json
import os
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# StonedGem has no keys of its own: products (OyNIx, PowerOyNix) keep their
# trusted-keys.txt; pass it with --keys. This default is for running inside OyNIx.
TRUSTED = ROOT / "packaging" / "poweroynix" / "trusted-keys.txt"
MAGIC = "OyNIx poweroynix v1\n"     # format 1: SHA-256 file hashes
MAGIC_V2 = "OyNIx poweroynix v2\n"  # format 2: SHA3-256
MAGIC_V3 = "OyNIx GemSig v3\n"      # format 3 (default): GemSig, SHA3-512
SIG_FILE = "oynix.sig"
MAX_BYTES = 64 << 20

# ── Ed25519 (RFC 8032, section 6) ─────────────────────────────────────
p = 2**255 - 19
q = 2**252 + 27742317777372353535851937790883648493


def _inv(x):
    return pow(x, p - 2, p)


d = -121665 * _inv(121666) % p
_sqrt_m1 = pow(2, (p - 1) // 4, p)


def _h(s):
    return int.from_bytes(hashlib.sha512(s).digest(), "little") % q


def _add(P, Q):
    A, B = (P[1] - P[0]) * (Q[1] - Q[0]) % p, (P[1] + P[0]) * (Q[1] + Q[0]) % p
    C, D = 2 * P[3] * Q[3] * d % p, 2 * P[2] * Q[2] % p
    E, F, G, H = B - A, D - C, D + C, B + A
    return (E * F, G * H, F * G, E * H)


def _mul(s, P):
    Q = (0, 1, 1, 0)
    while s > 0:
        if s & 1:
            Q = _add(Q, P)
        P = _add(P, P)
        s >>= 1
    return Q


def _equal(P, Q):
    return (P[0] * Q[2] - Q[0] * P[2]) % p == 0 and (P[1] * Q[2] - Q[1] * P[2]) % p == 0


def _recover_x(y, sign):
    if y >= p:
        return None
    x2 = (y * y - 1) * _inv(d * y * y + 1)
    if x2 == 0:
        return None if sign else 0
    x = pow(x2, (p + 3) // 8, p)
    if (x * x - x2) % p != 0:
        x = x * _sqrt_m1 % p
    if (x * x - x2) % p != 0:
        return None
    if (x & 1) != sign:
        x = p - x
    return x


_gy = 4 * _inv(5) % p
_gx = _recover_x(_gy, 0)
G = (_gx, _gy, 1, _gx * _gy % p)


def _compress(P):
    zi = _inv(P[2])
    x, y = P[0] * zi % p, P[1] * zi % p
    return int.to_bytes(y | ((x & 1) << 255), 32, "little")


def _decompress(s):
    if len(s) != 32:
        return None
    y = int.from_bytes(s, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = _recover_x(y, sign)
    return None if x is None else (x, y, 1, x * y % p)


def _expand(secret):
    h = hashlib.sha512(secret).digest()
    a = int.from_bytes(h[:32], "little")
    a &= (1 << 254) - 8
    a |= 1 << 254
    return a, h[32:]


def public_key(secret):
    return _compress(_mul(_expand(secret)[0], G))


def ed_sign(secret, msg):
    a, prefix = _expand(secret)
    A = _compress(_mul(a, G))
    r = _h(prefix + msg)
    Rs = _compress(_mul(r, G))
    s = (r + _h(Rs + A + msg) * a) % q
    return Rs + int.to_bytes(s, 32, "little")


def ed_verify(public, msg, sig):
    A = _decompress(public)
    R = _decompress(sig[:32]) if len(sig) == 64 else None
    if not A or not R:
        return False
    s = int.from_bytes(sig[32:], "little")
    if s >= q:
        return False
    return _equal(_mul(s, G), _add(R, _mul(_h(sig[:32] + public + msg), A)))


# ── Packages ──────────────────────────────────────────────────────────
def files_of(folder):
    out, total = [], 0
    for dirpath, dirnames, filenames in os.walk(folder, followlinks=False):
        for n in dirnames:
            if os.path.islink(os.path.join(dirpath, n)):
                sys.exit(f"{os.path.join(dirpath, n)} is a symbolic link; packages can't contain links")
        for n in filenames:
            full = Path(dirpath) / n
            if full.is_symlink():
                sys.exit(f"{full} is a symbolic link; packages can't contain links")
            if not full.is_file():
                continue
            rel = full.relative_to(folder).as_posix()
            if rel == SIG_FILE:
                continue
            if "\n" in rel:
                sys.exit("a file name contains a line break")
            total += full.stat().st_size
            if total > MAX_BYTES:
                sys.exit("the package is larger than 64 MB")
            out.append((rel, full))
    out.sort(key=lambda t: t[0].encode())
    return out


def identity(folder):
    """(id, version) the package says it is, from oynix.json."""
    try:
        meta = json.loads((Path(folder) / "oynix.json").read_text(encoding="utf-8-sig"))
    except (OSError, ValueError):
        meta = {}
    pid = meta.get("id") or meta.get("name") or ""
    ver = meta.get("version") or ""
    return str(pid), str(ver)


def _one_line(field, v):
    if "\n" in v or "\r" in v:
        raise ValueError(f"the {field} contains a line break")
    return v


def digest_message(folder, fmt=3, commit=""):
    """The signed message. Format 3 (GemSig) adds a header naming the package
    and the signing commit, and hashes files with SHA3-512."""
    if fmt >= 3:
        pid, ver = identity(folder)
        lines = [MAGIC_V3, f"id: {_one_line('id', pid)}\n", f"version: {_one_line('version', ver)}\n",
                 f"commit: {_one_line('commit', commit)}\n"]
        h = hashlib.sha3_512
    else:
        lines = [MAGIC_V2 if fmt == 2 else MAGIC]
        h = hashlib.sha3_256 if fmt == 2 else hashlib.sha256
    for rel, full in files_of(Path(folder)):
        lines.append(f"{h(full.read_bytes()).hexdigest()}  {rel}\n")
    return "".join(lines).encode()


def signed_bytes(folder, fmt=3, commit=""):
    """What Ed25519 signs: format 3 signs the message's SHA3-512 summary."""
    msg = digest_message(folder, fmt, commit)
    return hashlib.sha3_512(msg).digest() if fmt >= 3 else msg


# ── GemPrint (display only) ───────────────────────────────────────────
M64 = (1 << 64) - 1


def _rotl(x, k):
    return ((x << k) | (x >> (64 - k))) & M64


class Xoshiro256pp:
    def __init__(self, seed32):
        self.s = [int.from_bytes(seed32[8 * i:8 * i + 8], "little") for i in range(4)]
        if not any(self.s):
            self.s[0] = 1

    def next(self):
        s = self.s
        result = (_rotl((s[0] + s[3]) & M64, 23) + s[0]) & M64
        t = (s[1] << 17) & M64
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = _rotl(s[3], 45)
        return result


def _isqrt(n):
    import math
    return math.isqrt(n)


def gemprint_value(summary, commit):
    """round(H·H·√(seed+1) ÷ (1.5·x) ÷ c × 0.37125² ÷ (r+1)), exactly.

    H = SHA3-256(summary) as an integer; xoshiro256++ seeded from the summary
    draws seed ∈ 0..=3750, x (non-zero) and r ∈ 0..=100; c is the signing
    commit as a number. √ is taken in 64-bit fixed point; 0.37125² =
    88209/640000; the one rounding is half-up at the end."""
    H = int.from_bytes(hashlib.sha3_256(summary).digest(), "big")
    rng = Xoshiro256pp(summary[:32])
    seed = rng.next() % 3751
    x = rng.next() or 1
    r = rng.next() % 101
    try:
        cb = bytes.fromhex(commit) if len(commit) >= 16 else b""
    except ValueError:
        cb = b""
    if len(cb) < 8:
        cb = hashlib.sha3_256(commit.encode()).digest()
    c = int.from_bytes(cb[:8], "big") or 1
    num = H * H * _isqrt((seed + 1) << 128) * 2 * 88209
    den = 3 * x * c * 640000 * (r + 1) << 64
    return (2 * num + den) // (2 * den)


def gemprint(summary, commit):
    """The value's low 64 bits as four groups: a1b2-c3d4-e5f6-0718."""
    v = gemprint_value(summary, commit) & M64
    h = f"{v:016x}"
    return "-".join(h[i:i + 4] for i in range(0, 16, 4))


def trusted_keys(path):
    keys = {}
    for line in Path(path).read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        k, _, who = line.partition(" ")
        try:
            b = bytes.fromhex(k)
        except ValueError:
            continue
        if len(b) == 32:
            keys[b] = who.strip()
    return keys


def verify_dir(folder, keys):
    """Returns the key holder's name, or raises ValueError."""
    sig_path = Path(folder) / SIG_FILE
    if not sig_path.exists():
        raise ValueError("no oynix.sig: the package isn't verified")
    try:
        sig = json.loads(sig_path.read_text())
        key, signature = bytes.fromhex(sig["key"]), bytes.fromhex(sig["signature"])
    except (ValueError, KeyError, TypeError) as e:
        raise ValueError(f"oynix.sig is unreadable: {e}")
    if key not in keys:
        raise ValueError(f"signed by a key OyNIx doesn't trust ({key.hex()[:16]}…)")
    fmt = sig.get("format", 1)
    if fmt not in (1, 2, 3):
        raise ValueError(f"unknown signature format {fmt}")
    commit = sig.get("commit", "") if fmt >= 3 else ""
    if fmt >= 3:
        pid, ver = identity(folder)
        if sig.get("id", pid) != pid or sig.get("version", ver) != ver:
            raise ValueError(f"oynix.sig was made for {sig.get('id')} {sig.get('version')}, not {pid} {ver}")
    if not ed_verify(key, signed_bytes(folder, fmt, commit), signature):
        raise ValueError("the signature doesn't match the files: something changed after signing")
    who = keys[key] or key.hex()[:16]
    if fmt >= 3:
        who += f" (GemPrint {gemprint(signed_bytes(folder, 3, commit), commit)})"
    return who


def git_head(folder):
    import subprocess
    try:
        r = subprocess.run(["git", "-C", str(folder), "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10)
        return r.stdout.strip() if r.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError):
        return ""


def pack(folder, out):
    folder = Path(folder)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        names = [rel for rel, _ in files_of(folder)]
        if (folder / SIG_FILE).exists():
            names.append(SIG_FILE)
        for rel in sorted(names, key=str.encode):
            info = zipfile.ZipInfo(rel, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, (folder / rel).read_bytes())


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    k = sub.add_parser("keygen", help="make a signing key pair")
    k.add_argument("name", help="who holds the key (shown when OyNIx verifies a package)")
    k.add_argument("--out", help="private key file (default: <name>.poweroynix-key)")
    s = sub.add_parser("sign", help="sign a folder after reviewing it")
    s.add_argument("dir")
    s.add_argument("--key", required=True)
    s.add_argument("--pack", help="also write this .poweroynix")
    s.add_argument("--commit", help="the extension's source commit, recorded for GemPrint (default: git HEAD of DIR, if any)")
    s.add_argument("--format", type=int, default=3, choices=[2, 3], help="signature format (default 3, GemSig)")
    v = sub.add_parser("verify", help="check a folder or .poweroynix")
    v.add_argument("path")
    v.add_argument("--keys", default=str(TRUSTED))
    pk = sub.add_parser("pack", help="zip a signed folder into a .poweroynix")
    pk.add_argument("dir")
    pk.add_argument("out")
    a = ap.parse_args()

    if a.cmd == "keygen":
        out = Path(a.out or (a.name.lower().replace(" ", "-") + ".poweroynix-key"))
        if out.exists():
            sys.exit(f"{out} already exists; not overwriting a key")
        secret = os.urandom(32)
        pub = public_key(secret).hex()
        fd = os.open(out, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w") as f:
            json.dump({"name": a.name, "secret": secret.hex(), "public": pub}, f)
            f.write("\n")
        print(f"Private key: {out}  (keep it secret; never commit it)")
        print("Add this line to packaging/poweroynix/trusted-keys.txt:")
        print(f"{pub}  {a.name}")
    elif a.cmd == "sign":
        folder = Path(a.dir)
        meta_path = folder / "oynix.json"
        if not meta_path.exists():
            sys.exit(f"{folder}: oynix.json is missing")
        meta = json.loads(meta_path.read_text())
        if meta.get("power") is not True:
            sys.exit(f'{folder}: add "power": true to oynix.json before signing')
        key = json.loads(Path(a.key).read_text())
        secret = bytes.fromhex(key["secret"])
        pub = public_key(secret)
        fmt = a.format
        commit = ""
        if fmt >= 3:
            commit = a.commit if a.commit is not None else git_head(folder)
            _one_line("commit", commit)
        sig = ed_sign(secret, signed_bytes(folder, fmt, commit))
        out = {"format": fmt, "key": pub.hex(), "signature": sig.hex(), "signer": key.get("name", "")}
        if fmt >= 3:
            pid, ver = identity(folder)
            out.update({"id": pid, "version": ver, "commit": commit})
        (folder / SIG_FILE).write_text(json.dumps(out, indent=2) + "\n")
        print(f"Signed {meta.get('name', folder.name)} {meta.get('version', '')} as {key.get('name', pub.hex()[:16])} (format {fmt})")
        if fmt >= 3:
            print(f"GemPrint: {gemprint(signed_bytes(folder, 3, commit), commit)}")
        if a.pack:
            pack(folder, a.pack)
            print(f"Wrote {a.pack}")
    elif a.cmd == "verify":
        keys = trusted_keys(a.keys)
        path = Path(a.path)
        try:
            if path.is_dir():
                who = verify_dir(path, keys)
            else:
                with tempfile.TemporaryDirectory() as t, zipfile.ZipFile(path) as z:
                    for n in z.namelist():
                        if n.startswith("/") or ".." in Path(n).parts:
                            raise ValueError(f"unsafe path in archive: {n}")
                    z.extractall(t)
                    who = verify_dir(t, keys)
        except ValueError as e:
            sys.exit(f"{path}: NOT verified: {e}")
        print(f"{path}: verified by {who}")
    elif a.cmd == "pack":
        pack(a.dir, a.out)
        print(f"Wrote {a.out}")


if __name__ == "__main__":
    main()
