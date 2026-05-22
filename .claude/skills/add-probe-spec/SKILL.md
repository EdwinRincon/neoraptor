---
name: add-probe-spec
description: Create a new ProbeSpec<P> following ProbeProtocol trait (v0.2+)
disable-model-invocation: true
---

Generate a new ProbeSpec in `tools/`:

1. Ask user: target class (HTTP, DNS, TCP, etc.)
2. Ask user: probe name and expected indicators
3. Create `ProbeSpecDraft` struct with builder API
4. Define `ProbeStep` variants for the probe
5. Implement `ProbeProtocol` trait for the protocol
6. Add `ProbeSpecValidator` test case
7. Add migration for `ProbeSpecRepository` storage
8. Run `cargo test --package tools` and fix failures
9. Report: ProbeSpec created ✓ with probe ID and fingerprint