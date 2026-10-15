# Releasing

This page is for maintainers. A release ships four x86_64 packages: a Windows EXE, a Linux AppImage, a Debian/Ubuntu DEB and a Fedora RPM.

## Steps

1. Rename `Unreleased` in `CHANGELOG.md` to the new version and date, and add a new empty `Unreleased` section above it.
2. Set the same version in `package.json`, `package-lock.json`, `backend/Cargo.toml`, `backend/Cargo.lock` and `backend/tauri.conf.json`. Update the download links, sizes and version in `README.md` and `docs/MANUAL.md`.
3. Run the checks from [CONTRIBUTING.md](../CONTRIBUTING.md).
4. Commit, create the annotated tag `v<version>` on that commit, and push both.
5. The [Release workflow](../.github/workflows/release.yml) starts from the tag. It refuses to continue when any manifest version differs from the tag, builds the Windows EXE and the three Linux packages, writes `checksums.sha256`, and creates a **draft** GitHub Release with the notes from `CHANGELOG.md`.
6. Review the draft, check the files against `checksums.sha256`, and publish it.

Never move or reuse a release tag. Fix a bad release with a new version.

## Linux baseline

The workflow builds on `ubuntu-22.04`, which matches the glibc 2.35 baseline. DEB and RPM keep dependency declarations compatible with that baseline. Record the build distribution, required library versions and verified runtime distributions for each release separately; the Linux test bench record in `linux-testbench/VERIFICATION.md` covers only the versions it names.

## Building locally

`bin/build-linux`, `bin/build-windows` and `bin/build-macos` write versioned packages to `build/`, which Git ignores. Cargo keeps intermediate files in `build/.cache/`. Replace repeated builds of the same version and remove obsolete ones.

## Windows code signing

Without signing, Windows SmartScreen reports an unknown publisher. The Windows workflow can sign the EXE through [SignPath](https://signpath.io), which offers a free certificate for open-source projects through the SignPath Foundation. Check their current conditions before applying; they include a public repository with an open-source license and a code signing policy statement in the README.

After SignPath approves the project:

1. In SignPath, create the project and an artifact configuration that signs the portable executable, and a signing policy for the GitHub Actions build connector. The workflow defaults to the project slug `simpleVoiceover` and the policy slug `release-signing`.
2. In the GitHub repository settings, add the secret `SIGNPATH_API_TOKEN` and the variable `SIGNPATH_ORGANIZATION_ID`. Set `SIGNPATH_PROJECT_SLUG` or `SIGNPATH_SIGNING_POLICY_SLUG` only when your slugs differ from the defaults.
3. Run the Windows build workflow manually once. With the variable set, it uploads the unsigned EXE, submits it for signing, copies the signed file back and fails the job unless Windows reports a valid signature.
4. Replace the SmartScreen section of `README.md` with the SignPath attribution and the signing roles that SignPath requires.

While `SIGNPATH_ORGANIZATION_ID` is unset, the workflow publishes the unsigned EXE as before. Self-signed certificates do not remove the SmartScreen warning and are not used.

macOS builds are ad-hoc signed and not notarized. Notarization needs a paid Apple Developer account.
