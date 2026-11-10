---
name: release
description: Publishes a new Photoes version to GitHub and the AUR. Use when the user asks to release, tag, or publish a version, or when a release workflow run fails.
---

# Release Photoes

Release only when the user asks for it. A pushed tag publishes to the AUR within a minute and cannot be taken back.

The tag is the version the AUR gets. `Cargo.toml` is the version Nix gets. They must match: tag `v0.2.0` needs `version = "0.2.0"`.

1. Confirm `main` is clean and pushed.

   ```sh
   git status -sb
   ```

2. Set `version` in `Cargo.toml`.

3. Refresh `Cargo.lock`. The AUR build runs `cargo build --frozen` and fails when the lockfile holds the old version.

   ```sh
   nix develop --command cargo update --workspace
   ```

4. Check the package builds.

   ```sh
   nix build
   ```

5. Commit and push `Cargo.toml` and `Cargo.lock`.

6. Tag and push the tag.

   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z"
   ```

   ```sh
   git push origin vX.Y.Z
   ```

7. Watch `.github/workflows/aur.yml` finish.

   ```sh
   gh run watch --exit-status
   ```

8. Confirm the AUR copy shows the new `pkgver` and a real checksum.

   ```sh
   curl -s 'https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h=photoes' | grep -E '^(pkgver|sha256sums)'
   ```

The workflow publishes `aur/PKGBUILD` but does not build it. A green run does not prove the package builds on Arch.

When a run fails, read "Packaging and release" in `docs/design.md` before changing the workflow. The workflow needs the repo secret `AUR_SSH_PRIVATE_KEY`.
