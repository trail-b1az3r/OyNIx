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

The intended AUR maintainer/publisher account is **`ray0rfire`**.

1. Sign in to <https://aur.archlinux.org> as `ray0rfire`.
2. Generate a dedicated, passphrase-free SSH key for this workflow:

   ```bash
   ssh-keygen -t ed25519 -N '' -C 'oynix AUR publish (ray0rfire)' -f aur_oynix
   ```

3. While signed in as `ray0rfire`, paste `aur_oynix.pub` into **My Account → SSH Public Key**.
4. In this GitHub repository, replace the Actions secret `AUR_SSH_PRIVATE_KEY`
   with the contents of the new `aur_oynix` private key (not the `.pub` file):
   **Settings → Secrets and variables → Actions**.
5. Ensure `ray0rfire` is a maintainer of the existing `oynix-bin` AUR package.
   If it is currently maintained only by another account, add `ray0rfire` as a
   co-maintainer or transfer maintenance through the AUR package settings first.
6. Run **Actions → Publish to the AUR** once. The workflow's AUR commits are
   attributed to `ray0rfire`.

The workflow's SSH key determines which AUR account is authenticated; changing
the Git commit author alone does not change package ownership or maintainer access.
The repository secret must therefore contain a private key registered to
`ray0rfire`. GitHub Actions cannot read or replace that secret automatically.

Without the secret, automatic runs only warn and a manual run fails, saying why.

The workflow only trusts `aur.archlinux.org` host keys whose fingerprints match
the ones published on the AUR's home page. If the AUR ever rotates its keys,
update the three `SHA256:` fingerprints in `aur.yml`.

## Testing locally on Arch

```bash
cd packaging/aur/oynix-bin
updpkgsums && makepkg -si
```
