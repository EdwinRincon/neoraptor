# Observability

## Purpose and status

This document records proposed observability requirements for NEORAPTOR's
restricted M1 design. The repository is a scaffold; these controls and
diagnostics are not implemented or verified. M1 does not require distributed
actors, a Prometheus deployment or `/metrics` endpoint, OpenTelemetry exports,
Grafana/Alertmanager, centralized log aggregation, jemalloc or heap profiling,
tokio-console, or findings, escalation, or coverage dashboards.

See [milestones](./plan/INDEX.md) for staged scope and
[status](./plan/STATUS.md) for repository facts and unresolved decisions.
The [M1 protocol](./m1-protocol.md) specifies selected deployment, credential,
and operator-client behavior; the requirements here remain proposed.

## Distinct information classes

Keep these responsibilities separate:

1. **Authoritative accepted domain events** record accepted facts and state
   transitions. They follow persistence and ordering requirements; they are
   not diagnostic logs. If required storage is unavailable, do not claim a
   failure event can still be written.
2. **Best-effort bounded diagnostic logs** help explain operation and failure.
   They may be unavailable, incomplete, filtered, or lost. They are not an
   authoritative record or a substitute for accepted domain events.
3. **Operator inspection and readiness state** expose whether admission and
   dispatch are enabled, blocked, or degraded, and show relevant accepted
   actions and outcomes. Inspection is distinct from telemetry export.
4. **Optional aggregate instrumentation** may summarize resource and
   operational behavior if later justified. Its absence must not obscure
   authoritative facts or change authorization decisions.

## Required operator visibility

The design must provide appropriate operator visibility for:

- admission/readiness state and degraded or dispatch-disabled state;
- authenticated action outcomes without credential material;
- policy denials with stable reason codes, distinct from internal enforcement
  failures;
- request, attempt, run, policy, fixture-configuration, and relevant
  execution-configuration references where appropriate;
- broker execution-claim acquisition and authoritative final-check outcome;
- worker launch, stop, and unresolved cleanup;
- artifact/capture byte count and observation completeness;
- required persistence failures and their effect on new dispatch;
- cancellation requested versus worker stop confirmed;
- unknown execution outcomes and later actor-attributed reconciliation;
- bounded shutdown outcome and incomplete cleanup.

Browser connection/synchronization health is a separate signal from
backend-reported readiness and degraded state. Neither a live stream nor a
successful API response alone proves broker, persistence, worker, policy, or
network-enforcement health.

Expected policy rejection is a normal decision outcome, not automatically an
unrecoverable system error. Failure of an internal validation, authorization,
claim, or enforcement mechanism is a separate operational failure and must
fail closed. Diagnostics may report either condition, but must not convert a
denial into execution or imply persistence when storage is unavailable.

Use accepted domain events for durable domain facts where persistence permits.
Use bounded diagnostic records for operational context. Keep correlation
identifiers in suitable events or diagnostics subject to access control and
data-handling rules; do not make high-cardinality identifiers metric labels.

## Diagnostic and data-handling requirements

Diagnostic fields and messages must be bounded in length and count. Bound
queues and payloads, handle closure/receiver failure explicitly, and do not
silently shed required domain facts or outcomes. Best-effort logs may be
dropped or rotated according to an explicitly selected policy; no diagnostic
availability or durability guarantee is made.

Do not log:

- credentials, authorization headers, or other authentication secrets;
- URLs containing secret-bearing query or user-information components;
- response bodies by default;
- unrestricted tool output or raw sensitive artifacts.

Treat operator input, worker output, target content, and artifact metadata as
untrusted. Escape or remove terminal/control sequences so diagnostics do not
interpret them. Sanitize sensitive fields before emission, and avoid copying
raw artifact content into logs.

Log and telemetry storage needs bounded retention or rotation. The retention
policy, destination, storage mechanism, and numeric limits remain unresolved;
do not infer them from the per-artifact or per-run capture limits.

## Optional metrics

If aggregate metrics are later implemented, use only bounded enumerated label
sets. Never use run IDs, attempt IDs, operator identities, raw URLs, or
arbitrary error strings as labels. Prefer:

- **Counters** for cumulative occurrences, such as denial or worker-stop
  outcomes;
- **Gauges** for current bounded state, such as admission readiness or active
  execution-slot occupancy;
- **Histograms** only where duration or size distributions are useful and
  bounded measurements are defined.

Metric names, export protocol, instrumentation, collection, and alerting
remain implementation/deployment decisions. Metrics are optional summaries,
not an audit trail.

## Deferred capabilities and dependencies

Detailed distributed tracing, external telemetry backends, centralized
aggregation, alert rules, dashboards, coverage signals, heap profiling, and
development task-console tooling are deferred. They are not M1 prerequisites
and no related commands, paths, services, or infrastructure are claimed to
exist.

See [architecture](./architecture.md), [domain model](./domain-model.md),
[security](./security.md), [runtime](./runtime.md), and the
[proposed testing strategy](./testing.md) for the corresponding M1 contracts.
Persistence and deployment design must resolve event availability, bounded
diagnostic storage/retention, access, and failure behavior before operational
claims are made.
