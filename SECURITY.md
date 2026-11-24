# Security Policy

## Reporting a vulnerability

Report privately through GitHub: open the [Security tab](https://github.com/AntigmaLabs/ante/security) and choose **Report a vulnerability**. If that is not available to you, email contact@antigma.ai. Please do not open a public issue.

We aim to acknowledge a report within 2 business days and send an initial assessment within 7 days.

<!-- TODO(team): turn on private vulnerability reporting in the repository settings. It is off today, so the button does not appear yet. -->
<!-- TODO(team): replace contact@antigma.ai with a dedicated security address, and confirm the two targets above. -->

## Supported versions

The latest stable release and the latest nightly. Run `ante update` before you report.

## Scope

Any vulnerability in the `ante` binary or in the code in this repository. In particular:

- **Approval bypass.** In `strict` or `auto` permission mode, a tool call runs without the approval it should have asked for.
- **Telemetry beyond what the docs describe.** Ante exports something the [telemetry docs](https://docs.antigma.ai/configuration/preference#telemetry) do not list, or keeps exporting with `ANTE_TELEMETRY=off`.
