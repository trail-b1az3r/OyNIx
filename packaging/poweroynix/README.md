# `.poweroynix` (maintainers only)

A `.poweroynix` is an OyNIx extension with full power. The user-facing guide
is on the website (docs › Power extensions). It contains:

- the usual OyNIx files: `oynix.json` (with `"power": true`), `code.ts`,
  `content.css`, and optionally `graphics.qt`, `extra.html`, `extra.jsonl`
  and `theme.json`;
- optionally a Chrome or Firefox `manifest.json` (Manifest V2 or V3) and its
  files. Its background page, popup and content scripts run alongside;
- optionally `filters.txt` (Adblock/uBlock syntax), used with
  `"permissions": ["filters"]`. Its rules join OyNIx's content blocker while
  the extension is on;
- optionally `network.ts` (background: blocker rules, request headers,
  request feed, CORS-free fetch), `theme-shape.qml` (draws the browser's
  shapes), `extra.md` (a Markdown page) and `images/`;
- `oynix.sig`: the verification.

Because it's verified, it gets the powerful WebExtension APIs (native
messaging, webRequest, cookies, browsingData, management) without anyone
turning on Developer mode. Its manifest still has to ask for each permission.

## Verification

OyNIx installs a `.poweroynix` only when its `oynix.sig` is a valid Ed25519
signature over the SHA3-256 hash of every file in the package (signature
format 2, since 5.3.1; format 1 with SHA-256 is still accepted), made by a key in
`trusted-keys.txt` (compiled into OyNIx). The site build refuses to list one
in the gallery under the same rule. A package changed after signing fails.

Extensions in `builtin/` are the exception: they ship inside OyNIx, so they
need no signature (Wallbreaker is one). The gallery lists them without a
download, because a copy wouldn't verify anywhere else.

Signing **is** the review. Read every file first.

```bash
# once: make a key, keep the private file safe (never commit it)
python3 scripts/poweroynix.py keygen "Your Name"
#   → add the printed public line to packaging/poweroynix/trusted-keys.txt
#     (takes effect in the next OyNIx release)

# per extension, after review
python3 scripts/poweroynix.py sign gallery/some-extension --key your-name.poweroynix-key \
        --pack Some-Extension-1.0.0.poweroynix
python3 scripts/poweroynix.py verify Some-Extension-1.0.0.poweroynix
```

To test a power extension before it's signed, install the same files
without `"power": true`, as an ordinary OyNIx or Chrome extension. Then turn
on Settings › Extensions › Developer mode › Allow powerful extension APIs.
