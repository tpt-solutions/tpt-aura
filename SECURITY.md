# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of AURA seriously. If you discover a security vulnerability
(including issues that could break the provenance/trust-chain guarantees, or
allow tampered files to verify as authentic), please **do not open a public
GitHub issue**.

Instead, report it privately via one of the following:

- **GitHub Security Advisories:** use the "Report a vulnerability" button on the
  [Security tab](https://github.com/tpt-org/aura/security/advisories/new) of the
  repository. This keeps the report private until a fix is published.
- **Email:** send details to the maintainers (see `CODEOWNERS` for the responsible
  team). If available, use the PGP key published in the repository.

Please include:

1. A description of the vulnerability and its impact.
2. Steps to reproduce (proof-of-concept code or a crafted `.aura` file).
3. The affected crate(s) and version(s).

We will acknowledge receipt within **5 business days**, aim to provide a
remediation timeline within **14 days**, and coordinate a disclosure date with
you once a patch is available.

## Scope notes

AURA is a reference implementation. The embedded WASM bootstrap and the ONNX
backends (`aura-onnx`, feature `onnx`) depend on external toolchains/runtime;
vulnerabilities in those third-party components should be reported upstream.
