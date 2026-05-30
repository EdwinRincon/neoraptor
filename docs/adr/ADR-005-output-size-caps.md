# ADR-005: Output Size Caps

**Status:** Accepted

**Date:** 2026-05-31

**Context:**

Security tools can produce unbounded output:
- Full packet captures (GB of data)
- Large file dumps (vulnerability scanner output)
- Verbose debug logs (exploit framework traces)

Without caps, we risk:

1. **Memory exhaustion**: Buffering large outputs in memory
2. **Storage exhaustion**: Writing unbounded artifacts to disk
3. **Event log bloat**: Embedding large artifacts in events

**Decision:**

Enforce strict **size caps** on all sandbox output:

1. **Per-artifact cap**: 10 MB (configurable)
2. **Per-run cap**: 100 MB (configurable)
3. **Exceeded cap behavior**: Truncate content + emit `ArtifactTruncated` warning event

Caps are enforced during **streaming ingestion** (not buffering full output then checking size).

**Consequences:**

**Positive:**
- **Bounded resource usage**: Memory and storage are protected
- **Fast failure**: Caps are checked during streaming, not after full ingestion
- **Visibility**: `ArtifactTruncated` events expose cap violations
- **Operator override**: Caps are configurable per run (can raise for specific tools)

**Negative:**
- **Data loss**: Truncated artifacts may lose critical evidence
- **Tool-specific tuning**: Some tools (e.g., full packet capture) require higher caps
- **Streaming complexity**: Cap enforcement must happen during streaming, not buffering

**Alternatives Considered:**

1. **No caps**: Simplest but risks resource exhaustion
2. **Post-hoc truncation**: Buffer full output, then truncate → memory spike
3. **Compression**: Compress artifacts before capping → adds CPU overhead, still needs a cap

**Implementation Notes:**

**Streaming ingestion:**

```rust
impl Executor {
    async fn ingest_with_cap(
        &self,
        mut stream: impl Stream<Item = Bytes>,
        cap_bytes: u64,
    ) -> Result<EvidenceArtifact, ProbeError> {
        let mut buffer = Vec::new();
        let mut total_bytes = 0;
        let mut truncated = false;

        while let Some(chunk) = stream.next().await {
            if total_bytes + chunk.len() as u64 > cap_bytes {
                // Cap exceeded: truncate and stop streaming
                let remaining = (cap_bytes - total_bytes) as usize;
                buffer.extend_from_slice(&chunk[..remaining]);
                truncated = true;
                break;
            }
            buffer.extend_from_slice(&chunk);
            total_bytes += chunk.len() as u64;
        }

        if truncated {
            emit_event(ArtifactTruncated { size_bytes: total_bytes, cap_bytes });
        }

        Ok(EvidenceArtifact {
            content: buffer,
            size_bytes: total_bytes,
            truncated,
        })
    }
}
```

**Configurable caps:**

```rust
let run_config = RunConfig {
    artifact_cap_bytes: 10 * 1024 * 1024,  // 10 MB per artifact
    run_cap_bytes: 100 * 1024 * 1024,      // 100 MB per run
};
```

**Metrics:**
- `neoraptor_artifacts_truncated_total` (counter)
- `neoraptor_artifact_size_bytes` (histogram)

**References:**

- [security.md](../security.md) — Output caps and sandbox guarantees
- [domain-model.md](../domain-model.md) — EvidenceArtifact structure
- [observability.md](../observability.md) — Artifact size metrics
