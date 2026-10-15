# Security policy

## Supported versions

Security fixes are made for the latest release. Please reproduce a problem on the newest version before reporting it.

## Reporting a vulnerability

Do not open a public issue for a security problem. Use **[Report a vulnerability](https://github.com/arconw/simpleVoiceover/security/advisories/new)** on the repository's Security tab. The report stays private between you and the maintainer until a fix is available.

Please include the version, the operating system, the steps to reproduce, and what an attacker could achieve.

## Scope

simpleVoiceover is a desktop application. Its engine listens on `127.0.0.1` only. Reports about these areas are especially useful:

- The local HTTP and WebSocket endpoints, including origin checks.
- Parsing of imported media and of project files.
- File access outside the folders the user chose.
- The integrity of released packages and their checksums.

Windows executables are not code-signed yet, so SmartScreen warns on first launch. That warning alone is not a vulnerability. Always compare downloads with the `checksums.sha256` file attached to the release.
