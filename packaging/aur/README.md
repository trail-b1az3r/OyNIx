# AUR: `oynix-bin`

`oynix-bin/PKGBUILD` repackages the prebuilt `oynix-<version>-1-x86_64.pkg.tar.zst`
from a GitHub release, so AUR users get exactly the files the release ships (with
OyNIx's own Qt under `/usr/lib/oynix`) without a long build. It `provides` and
`conflicts` with `oynix`.

`.github/workflows/aur.yml` keeps it current. For each stable release it:

1. downloads the release's package and checks it against the release's `SHA256SUMS`;
2. writes the version, `pkgrel` and real checksum into the PKGBUILD;
3. builds it with `makepkg` in an Arch container and writes `.SRCINFO`;
4. pushes `PKGBUILD` and `.SRCINFO` to `ssh://aur@aur.archlinux.org/oynix-bin.git`.

It runs on its own after `build.yml` publishes a release (pre-releases and drafts
are skipped) and whenever a release is published by hand. You can also run it from
**Actions → Publish to the AUR**: leave the version empty for the latest release,
raise `pkgrel` to ship a packaging fix for the same version, or tick **dry run** to
check the package without pushing.

## One-time setup

1. Create an account on <https://aur.archlinux.org> (or use the maintainer's).
2. Make a key just for this workflow, with no passphrase:

   ```bash
   ssh-keygen -t ed25519 -N '' -C 'oynix AUR publish' -f aur_oynix
   ```

3. Paste `aur_oynix.pub` into **My Account → SSH Public Key** on the AUR.
4. In this repository, add `aur_oynix` (the private half) as an Actions secret
   named `AUR_SSH_PRIVATE_KEY` (**Settings → Secrets and variables → Actions**).
   Then delete the local copy.
5. Run **Actions → Publish to the AUR** once. The first push creates the
   `oynix-bin` package with that account as its maintainer.

Without the secret, automatic runs only warn and a manual run fails, saying why.

The workflow only trusts `aur.archlinux.org` host keys whose fingerprints match
the ones published on the AUR's home page. If the AUR ever rotates its keys,
update the three `SHA256:` fingerprints in `aur.yml`.

## Testing locally on Arch

```bash
cd packaging/aur/oynix-bin
updpkgsums && makepkg -si
```
