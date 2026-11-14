# Extension Gallery

Each folder here is one OyNIx extension, published at
<https://trail-b1az3r.github.io/OyNIx/extensions/> by `scripts/build-site.py`.

An extension is `oynix.json`, `content.js` and `content.css`, plus a
`gallery` section in `oynix.json` for its listing. The format and the review
rules are in the docs: <https://trail-b1az3r.github.io/OyNIx/docs/build-an-extension/>.

Check your extension installs before opening a pull request:

```bash
oynix --install-extension gallery/your-extension
python3 scripts/build-site.py && python3 -m http.server -d _site 8000
```
