# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in zig, please report it responsibly using [GitHub's private vulnerability reporting](https://github.com/niclaslindstedt/zig/security/advisories/new). Do not open a public issue.

Please include:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- The affected zig version
- Any relevant configuration or environment details

### Response timeline

- **Acknowledgment**: within 2 business days
- **Severity assessment**: within 7 days
- **Patch release**: within 90 days of confirmed vulnerabilities

## Scope

This policy covers the `zig` CLI tool and the `zig-core` library crate. Security issues in upstream tools (zag, Claude, Codex, Gemini, Copilot, Ollama) should be reported to their respective maintainers.

## Safe harbor

We support security research conducted in good faith. You are authorized to test for vulnerabilities as long as you:

- Avoid privacy violations and disruption of services
- Only test on accounts you own or have explicit consent for
- Disclose findings promptly and allow reasonable remediation time

## Disclosure policy

We follow a 90-day coordinated disclosure window. Once a patch is released we will publish a GitHub security advisory. Researchers who report privately are credited in the advisory unless they prefer to remain anonymous. If a fix requires longer than 90 days we will communicate that directly with the reporter.

## Supported versions

Only the latest release is fully supported. Older versions receive best-effort attention. We recommend keeping your installation up to date.
