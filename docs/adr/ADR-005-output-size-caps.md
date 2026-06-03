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
**Memory efficiency improvements:**

1. **Pre-allocation**: Use `Vec::with_capacity(cap_bytes)` (maximum buffer size in bytes) instead of `Vec::new()` to avoid growth reallocations during chunk ingestion  
2. **Ref-counted storage**: Replace `content: Vec<u8>` with `content: Bytes` (from the `bytes` crate) to enable O(1) ref-counted cloning across actors without duplicating payloads (immutable, shared buffer)  
3. **Event delegation**: Return `Result<IngestResult, _>` where `IngestResult` contains both the artifact and a `Vec<RunEvent>` of deferred events; the caller is responsible for emitting them, ensuring they are not dropped under event-store backpressure  

**Updated implementation:**

```rust
use bytes::Bytes;

struct IngestResult {
    artifact: EvidenceArtifact,
    deferred_events: Vec<RunEvent>, // Caller must emit these
}

// Replace line:
// let mut buffer = Vec::new();
// With:
let mut buffer = Vec::with_capacity(cap_bytes);

// Replace lines (emit_event call)
// With:
let mut deferred_events = Vec::new();
if truncated {
    deferred_events.push(RunEvent::ArtifactTruncated {
        size_bytes: total_bytes,
        cap_bytes,
    });
}

// Replace return type and value:
Ok(IngestResult {
    artifact: EvidenceArtifact {
        content: Bytes::from(buffer), // Single move into Bytes; no further copies, cheap clones after
        size_bytes: total_bytes,
        truncated,
    },
    deferred_events,
})
```

**Rationale:**

- Pre-allocation avoids growth reallocations when ingesting large streams (e.g., ~10 MB)  
- `Bytes` provides immutable, shared storage with O(1) cloning across actors (e.g., analyzer, archiver)  
- Returning deferred events ensures truncation signals are not dropped due to emitter backpressure; the caller controls emission and retry semantics

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
