# Observability

This document describes NEORAPTOR's observability strategy: metrics, traces, logs, coverage signals, heap health monitoring, and degradation detection.

## Overview

NEORAPTOR is a long-running, distributed actor system. Observability is essential for understanding system behavior, detecting degradation, and debugging production issues.

**Core observability principles:**

1. **Metrics for trends** — Prometheus-compatible metrics for dashboards and alerts
2. **Traces for causality** — Distributed tracing for run → probe → finding lineage
3. **Logs for context** — Structured JSON logs with run/probe/actor IDs
4. **Events for state** — Append-only event log captures all state changes
5. **Coverage for completeness** — Coverage map shows tested vs untested regions

## Metrics

NEORAPTOR exposes Prometheus-compatible metrics at `/metrics`.

### Run Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_runs_total` | Counter | Total runs initiated |
| `neoraptor_runs_active` | Gauge | Currently active runs |
| `neoraptor_runs_completed_total` | Counter | Completed runs (by status: success, aborted) |
| `neoraptor_run_duration_seconds` | Histogram | Run duration from initiation to completion |

**Labels:** `status` (success, aborted), `operator`

### Probe Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_probes_dispatched_total` | Counter | Probes dispatched to execution plane |
| `neoraptor_probes_completed_total` | Counter | Probes completed (by status: success, failed) |
| `neoraptor_probes_failed_total` | Counter | Probes failed (by error type: timeout, policy, resource) |
| `neoraptor_probe_duration_seconds` | Histogram | Probe duration from dispatch to completion |
| `neoraptor_probe_retry_count` | Histogram | Number of retries per probe |

**Labels:** `tool_family`, `vulnerability_class`, `status`, `error_type`

### Finding Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_findings_confirmed_total` | Counter | Confirmed findings (by severity) |
| `neoraptor_false_positives_total` | Counter | False positives filtered by validator |
| `neoraptor_finding_confirmation_rate` | Gauge | Ratio of confirmed findings to total probes |

**Labels:** `severity` (critical, high, medium, low, info), `vulnerability_class`

### Escalation Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_escalations_proposed_total` | Counter | Escalations proposed by chain planner |
| `neoraptor_escalations_approved_total` | Counter | Escalations approved by mentor |
| `neoraptor_escalations_disputed_total` | Counter | Escalations disputed by mentor |
| `neoraptor_escalation_review_duration_seconds` | Histogram | Time from proposal to approval/dispute |

**Labels:** `state` (approved, disputed, superseded)

### Actor Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_actor_mailbox_depth` | Gauge | Current depth of actor mailbox queue |
| `neoraptor_actor_messages_processed_total` | Counter | Messages processed by actor |
| `neoraptor_actor_panics_total` | Counter | Panics captured by supervisor |
| `neoraptor_backpressure_triggered_total` | Counter | Backpressure events (queue full) |
| `neoraptor_queue_saturation_total` | Counter | Queue saturation transitions (shed mode entered/exited) |

**Labels:** `actor_type` (orchestrator, planner, executor, validator, chain_planner, mentor, supervisor)

**Queue Saturation Events:**

In addition to the Prometheus counter, a `QueueSaturation` run event is emitted whenever shedding begins or ends. This allows:

- Replay to reconstruct when saturation occurred and correlate it with gaps in probing
- Operators to distinguish transient saturation spikes from sustained overload
- Incident review to trace saturation to specific runs or planning decisions

The Prometheus counter remains the live alert signal; the event log provides the durable audit trail.

### Sandbox Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_sandbox_spawn_duration_seconds` | Histogram | Time to spawn sandbox |
| `neoraptor_sandbox_cpu_usage_seconds` | Counter | CPU time consumed by sandboxes |
| `neoraptor_sandbox_memory_bytes` | Gauge | Memory usage by active sandboxes |
| `neoraptor_sandbox_network_bytes_sent` | Counter | Network bytes sent from sandboxes |
| `neoraptor_sandbox_network_bytes_received` | Counter | Network bytes received by sandboxes |

**Labels:** `sandbox_backend` (docker, gvisor, firecracker)

### Coverage Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_coverage_port_range` | Gauge | Coverage ratio for port ranges (0.0 to 1.0) |
| `neoraptor_coverage_subnet` | Gauge | Coverage ratio for subnet ranges (0.0 to 1.0) |
| `neoraptor_coverage_vulnerability_class` | Gauge | Coverage ratio per vulnerability class (0.0 to 1.0) |

**Labels:** `dimension` (port_range, subnet, vulnerability_class)

### Heap Health Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neoraptor_heap_size_bytes` | Gauge | Total heap size |
| `neoraptor_heap_growth_rate_bytes_per_sec` | Gauge | Rate of heap growth |
| `neoraptor_heap_allocated_bytes` | Gauge     | Total heap allocated bytes (jemalloc stats.allocated) |
| `neoraptor_heap_active_bytes`     | Gauge     | Active heap bytes (jemalloc stats.active)              |
| `neoraptor_heap_resident_bytes`   | Gauge     | Resident heap bytes (jemalloc stats.resident)          |

**Alerts:**
- Heap growth > 10 MB/minute for 5 minutes → warning
- Heap size > 1 GB → critical

## Traces

NEORAPTOR uses OpenTelemetry for distributed tracing. Traces capture the full lineage of a run from initiation to completion.

### Span Hierarchy

```
run (root span)
  ├─ orchestrate
  ├─ plan
  │   ├─ generate_probe (per probe)
  │   └─ validate_probe (per probe)
  ├─ execute
  │   ├─ spawn_sandbox
  │   ├─ run_tool
  │   └─ ingest_output
  ├─ validate
  │   ├─ parse_output
  │   └─ confirm_finding
  └─ chain_plan
      ├─ synthesize_chain
      └─ propose_escalation
```

### Trace Context

Every span includes:

- `run_id`: Parent run identifier
- `authorization_id`: ScopeContract authorization
- `operator`: Who initiated the run
- `tool_family`: Tool being executed (if applicable)
- `vulnerability_class`: Probe type (if applicable)

**Trace export:** Traces are exported to OTLP-compatible backends (Jaeger, Honeycomb, Grafana Tempo).

### Example Query

"Show me all probes for run X that took > 10 seconds":

```promql
histogram_quantile(0.95, 
  sum(rate(neoraptor_probe_duration_seconds_bucket{run_id="X"}[5m])) by (le)
) > 10
```

## Logs

NEORAPTOR emits structured JSON logs to stdout. Logs are captured by the container runtime and forwarded to a centralized log aggregator (e.g., Loki, Elasticsearch).

### Log Format

```json
{
  "timestamp": "2026-05-31T12:34:56.789Z",
  "level": "INFO",
  "message": "Probe dispatched",
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "probe_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "tool_family": "PortScanner",
  "target": "192.168.1.10",
  "authorization_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6"
}
```

### Log Levels

- `ERROR`: Unrecoverable errors (policy violations, panics)
- `WARN`: Recoverable errors (probe failures, output caps exceeded)
- `INFO`: State transitions (probe dispatched, finding confirmed)
- `DEBUG`: Detailed execution (sandbox spawn, artifact ingestion)
- `TRACE`: Per-message actor logging (for debugging only)

**Production level:** `INFO` (configurable via `RUST_LOG`)

### Correlated Logs

Logs are correlated via `run_id` and `probe_id`:

**Query:** "Show me all logs for run X":

```logql
{app="neoraptor"} |= "550e8400-e29b-41d4-a716-446655440000"
```

## Events

The append-only event log captures all state changes. Events are **not** logs — they are the source of truth.

**Differences:**
- **Logs**: Diagnostic output, may be sampled or filtered
- **Events**: State changes, never sampled, always persisted

**Query:** Event queries are implemented via the event-store API (SQL queries over the event log table).

**See:** [domain-model.md](./domain-model.md) for event schema.

## Coverage Signals

The coverage map tracks what has been tested and what remains. Coverage metrics are updated incrementally as probes complete.

### Coverage Dimensions

1. **Port ranges**: Which ports have been scanned
2. **Subnet ranges**: Which IP addresses have been tested
3. **Vulnerability classes**: Which attack types have been attempted
4. **Tool families**: Which tools have been used

### Coverage Visualization

The UI renders coverage as a heatmap:

- **Red**: Untested (0% coverage)
- **Yellow**: Partial (1-99% coverage)
- **Green**: Full (100% coverage)

**Example:** After scanning ports 1-1000 on `192.168.1.0/24`:

```
Coverage by port range (192.168.1.0/24):
  1-1000:     ████████████████████ 100% (green)
  1001-65535: ░░░░░░░░░░░░░░░░░░░░   0% (red)
```

### Degradation Signals

Coverage degradation (e.g., "we used to test X but stopped") is detected via historical metrics:

**Query:** "Did port coverage decrease in the last 24 hours?"

```promql
delta(neoraptor_coverage_port_range[24h]) < 0
```

**Alert:** Coverage drop > 10% in 24 hours → warning

## Heap Health Monitoring

Long-running actor systems can leak memory. NEORAPTOR monitors heap health and exposes dev tooling for leak detection.

### Monitoring Strategy

**Metrics:**
- `heap_size_bytes`: Total heap size
- `heap_growth_rate_bytes_per_sec`: Rate of heap growth
- `actor_mailbox_depth`: Per-actor queue depth

**Alerts:**
- Heap growth > 10 MB/minute for 5 minutes → warning
- Heap size > 1 GB → critical

**Heap growth detection:**

```promql
rate(neoraptor_heap_size_bytes[5m]) > 10 * 1024 * 1024
```

### Dev Tooling

**Heap snapshot:**

```bash
just heap-snapshot
```

Captures a heap snapshot using `pprof` (requires `jemalloc` allocator). Output is saved to `heap-snapshot.pb.gz` and can be analyzed with `pprof`:

```bash
pprof -http=:8080 heap-snapshot.pb.gz
```

**tokio-console:**

```bash
just tokio-console
```

Launches `tokio-console` for real-time task inspection. Requires `RUSTFLAGS="--cfg tokio_unstable"` at build time.

**Memory profiling:**

```bash
cargo build --release --features=jemalloc
MALLOC_CONF=prof:true ./target/release/neoraptor-api
```

Enables `jemalloc` profiling. Heap dumps are written to `jeprof.*.heap`.

## Alerting

NEORAPTOR integrates with Prometheus Alertmanager for critical alerts.

### Alert Rules

| Alert | Condition | Severity |
|-------|-----------|----------|
| `HighProbeFailureRate` | Probe failure rate > 10% for 5 minutes | Warning |
| `ActorMailboxFull` | Actor mailbox depth > 90% capacity | Critical |
| `HeapGrowthExceeded` | Heap growth > 10 MB/minute for 5 minutes | Warning |
| `HeapSizeExceeded` | Heap size > 1 GB | Critical |
| `PanicRateHigh` | Panic rate > 1/minute for 5 minutes | Critical |
| `CoverageDrop` | Coverage drop > 10% in 24 hours | Warning |

**Notification channels:** PagerDuty, Slack, email

## Dashboards

NEORAPTOR provides pre-built Grafana dashboards:

1. **Run Overview**: Run count, duration, completion rate
2. **Probe Execution**: Probe dispatch rate, failure rate, retry count
3. **Finding Summary**: Confirmed findings by severity and vulnerability class
4. **Actor Health**: Mailbox depth, backpressure events, panic rate
5. **Sandbox Metrics**: Spawn latency, CPU/memory usage, network traffic
6. **Coverage Map**: Coverage by dimension (ports, subnets, vulnerability classes)
7. **Heap Health**: Heap allocations, active bytes, resident memory (via jemalloc)

**Import:** Dashboards are in `deploy/grafana/dashboards/`.

## Performance Profiling

### CPU Profiling

```bash
cargo build --release
perf record -F 99 -g ./target/release/neoraptor-api
perf report
```

**Flamegraph:**

```bash
cargo install flamegraph
cargo flamegraph --bin neoraptor-api
```

### Async Task Profiling

```bash
RUSTFLAGS="--cfg tokio_unstable" cargo build --release
tokio-console http://localhost:6669
```

**Blocked tasks:** Look for tasks with high `Busy` time or `Poll` count.

## Further Reading

- [architecture.md](./architecture.md) — High-level system overview
- [runtime.md](./runtime.md) — Actor supervision and backpressure
- [domain-model.md](./domain-model.md) — Event schema and coverage model
- [security.md](./security.md) — Security metrics and audit
- [testing.md](./testing.md) — Observability in tests (metrics, traces)
