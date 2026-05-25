---
title: Language Support and LSP Shutdown
---

## Why did language work affect Helix shutdown?

Short version: two things were tied together that should have stayed separate.

When we added language-specific behavior (automatic language detection, language-specific dictionaries, delayed diagnostics), we also increased how much work happened right before and during editor close.

Helix closes an LSP server in a strict order:

1. send `shutdown`
2. wait for a response
3. send `exit`

If the server is still busy with other work (for example delayed diagnostics or cleanup that is still running), the `shutdown` response can arrive too late. Helix then logs a timeout.

So the bug was not “German support is bad by itself”. The bug was that language-related work and shutdown handling shared the same runtime path and could block each other.

## Why this can be confusing

From a user perspective, it feels unrelated:

- “I only changed language support.”
- “Why does close behavior break?”

But in an LSP server, all of this still runs inside one process. If shutdown is not treated as a strict, high-priority path, unrelated features can accidentally delay it.

## What we changed

- Added explicit progress lifecycle cleanup for delayed diagnostics (so in-flight work is properly finished/canceled).
- Moved language-specific parser/dictionary/linter wiring behind a central language registry.
- Added Portuguese as a first-class dialect value in the shared dialect model.
- Added shared compressed dictionary loading code, so language dictionaries follow one loading pattern.

## How to separate this better going forward

Use this design rule:

**Language features may affect _what_ diagnostics are produced, but never _whether shutdown can complete quickly_.**

Practical guardrails:

1. Shutdown path must only do constant-time bookkeeping and reply immediately.
2. Any pending background analysis must be cancelable and must not block shutdown.
3. Language selection should be configuration/state lookup, not ad-hoc branching spread across subsystems.
4. Runtime features (progress, delays, retries) should be tested independently from language correctness tests.

## Suggested follow-up

Introduce a dedicated “shutdown coordinator” in `harper-ls` that owns cancellation of all pending analysis tasks, so feature code only registers work and never controls shutdown directly.
