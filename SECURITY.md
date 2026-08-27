# Security Policy

Plugins execute with the host user's permissions and are intentionally not
sandboxed or signature-verified. Do not report malicious packages or
exploitable vulnerabilities in a public issue. Use a private GitHub Security
Advisory and include the affected plugin id/version, reproduction steps and
logs with secrets removed.

Known-bad versions may be listed in `market/security/revocations.json` as a
manual takedown notice. The PetAgent client does not treat this file as a
cryptographic revocation mechanism; remove the release from the index and
disable or uninstall affected plugins.
