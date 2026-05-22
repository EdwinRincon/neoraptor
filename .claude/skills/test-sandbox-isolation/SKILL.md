---
name: test-sandbox-isolation
description: Integration test verifying sandbox cannot reach control-net
disable-model-invocation: true
---

Verify sandbox network isolation per ADR-002:

1. Start `docker compose up -d` (control-net + execution-net)
2. Run integration test: sandbox container attempts DB connection
3. Assert connection fails (must timeout or be blocked)
4. Run sandbox tool execution test (must succeed on execution-net)
5. Report: isolation verified ✓ or show breach with logs