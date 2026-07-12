# Security Policy

## Supported Versions

This project is early-stage. Security fixes target the latest `main` branch until formal releases are published.

## Reporting a Vulnerability

Please do not open a public issue for vulnerabilities involving crafted binaries, terminal escape handling, or unsafe file processing.

Report privately by contacting the maintainers through the repository security advisory flow when available. If advisories are not enabled yet, open a minimal public issue asking for a private contact path without including exploit details.

Useful report details:

- Affected commit or release.
- Platform and terminal used.
- Minimal input file or reproduction steps, shared privately.
- Expected impact, such as crash, terminal control sequence injection, path overwrite, or denial of service.

## Scope

Relevant security areas include binary parsing, disassembly of untrusted files, export path handling, terminal escape rendering, and project-file loading.