# P2-T8 CI/CD Handoff Contract + Machine-Readable Execution Outputs Design

## Goal

Define the narrowest useful CI/CD handoff contract for TestForge that produces deterministic machine-readable execution output, exposes stable `passed` / `failed` / `blocked` semantics, and reuses the existing runner/orchestration/artifact model without expanding into a full CLI platform or remote execution control plane.

## Context

P2-T8 follows the completion of P2-T7 local scheduling and must preserve the same core architectural boundaries already established in Phase 2:

- frontend/backend communication remains behind typed IPC contracts
- suite execution remains driven by the existing runner orchestration pipeline
- large payloads stay filesystem-backed through artifact services, not SQLite blobs
- all new outputs remain redacted by default per P2-T5 security hardening

The user explicitly approved the following product direction for this task:

- use a **minimal hybrid** design
- treat **`blocked` as a separate exit code**
- support **one suite execution result per handoff** as the baseline
- treat a **JSON file on disk as the canonical contract artifact**

## Problem Statement

TestForge currently has strong internal runner, reporting, scheduling, and artifact seams, but no dedicated handoff contract that an external CI/CD pipeline can consume deterministically. P2-T8 must add that handoff layer without creating a second execution model, a new reporting backend, or a headless platform rewrite.

## Constraints

### Must Have

- deterministic machine-readable output that external pipelines can parse
- stable high-level exit semantics for `passed`, `failed`, and `blocked`
- output artifact that remains consistent with existing persisted run results and artifact manifests
- secret-safe serialization that preserves Phase 2 redaction guarantees
- reuse of existing runner orchestration and filesystem artifact policy

### Must Not Do

- do not turn this into a full CLI platform or remote execution control plane
- do not create a parallel execution model separate from the current runner pipeline
- do not break the typed frontend/backend contract just to satisfy CI integration
- do not expose raw secrets in artifacts, logs, diagnostics, or response payloads
- do not broaden scope to multi-suite batching in the baseline

## Architecture

P2-T8 uses a **minimal hybrid** handoff model:

1. a typed invocation boundary triggers execution for exactly one suite
2. backend orchestration reuses the existing runner pipeline to execute the suite
3. backend normalizes the final result into a CI handoff projection
4. that projection is persisted as a deterministic JSON artifact on disk
5. the invocation returns a thin response with status metadata and a pointer to the JSON artifact

This keeps the execution truth inside the current orchestration and persistence model, while giving CI callers a stable contract they can archive, parse, and branch on.

## Execution Flow

1. caller submits a CI handoff request for one suite
2. backend validates the suite reference and runtime prerequisites
3. runner orchestration executes the suite through the normal run pipeline
4. persisted run/result/artifact data is collected from the shared repository/read model
5. backend maps the final state into CI handoff semantics
6. artifact service writes a canonical JSON artifact to disk using the established artifact path policy
7. invocation returns a minimal response containing `run_id`, `suite_id`, final status, and artifact path

## Contract Shape

### Request

The baseline request should stay intentionally narrow:

```json
{
  "suiteId": "string",
  "trigger": {
    "source": "ci",
    "actor": "pipeline",
    "label": "optional human-readable pipeline label"
  },
  "output": {
    "writeJson": true,
    "outputDir": "optional explicit directory",
    "fileName": "optional deterministic file name"
  }
}
```

#### Request Rules

- `suiteId` is required
- the baseline accepts exactly one existing suite per invocation
- `trigger.source` is fixed to `ci` in this phase
- `output.writeJson` must remain true because the disk artifact is the canonical contract
- `outputDir` is optional and must still resolve inside an approved export/artifact root
- no batching, sharding, distributed worker hints, remote targets, or advanced retry controls

### Thin Invocation Response

The invocation response is deliberately smaller than the full result payload:

```json
{
  "status": "passed | failed | blocked",
  "artifactPath": "absolute-or-app-resolved-path",
  "runId": "string",
  "suiteId": "string"
}
```

This response exists only to tell the caller where the canonical JSON artifact lives and what the high-level outcome was. It must not become a second full result schema.

## Canonical JSON Artifact

### Design Goals

The JSON artifact is the canonical CI handoff contract. It must be:

- deterministic
- machine-readable
- schema-versioned
- secret-safe
- aligned to persisted run results and artifact metadata
- rich enough for CI decisions without requiring direct runtime introspection

### Proposed v1 Schema

```json
{
  "schemaVersion": "1",
  "contractType": "testforge.ci.execution-result",
  "generatedAt": "2026-04-03T12:34:56.000Z",
  "status": "passed",
  "exitCode": 0,
  "run": {
    "runId": "run_123",
    "suiteId": "suite_456",
    "suiteName": "Smoke Suite",
    "triggerSource": "ci",
    "triggerActor": "pipeline",
    "startedAt": "2026-04-03T12:33:10.000Z",
    "finishedAt": "2026-04-03T12:34:56.000Z",
    "durationMs": 106000
  },
  "summary": {
    "totalTargets": 4,
    "passedTargets": 4,
    "failedTargets": 0,
    "blockedTargets": 0,
    "cancelledTargets": 0
  },
  "failure": null,
  "artifacts": [
    {
      "artifactId": "artifact_1",
      "kind": "report_json",
      "path": "C:\\Users\\...\\exports\\ci\\run_123.json",
      "relativePath": "exports\\ci\\run_123.json"
    }
  ],
  "redaction": {
    "applied": true,
    "policyVersion": "phase2-default",
    "notes": [
      "Sensitive headers masked",
      "Secret-backed variables omitted or redacted"
    ]
  }
}
```

### Required Fields

At minimum, v1 must always provide:

- `schemaVersion`
- `contractType`
- `generatedAt`
- `status`
- `exitCode`
- `run.runId`
- `run.suiteId`
- `summary`
- `artifacts`
- `redaction.applied`

### Failure Object

When `status` is `failed` or `blocked`, the artifact must include a structured failure object:

```json
{
  "kind": "assertion | transport | preflight | runtime_blocked | orchestration",
  "code": "STRING_CODE",
  "message": "Human-readable summary",
  "details": {
    "targetId": "optional",
    "targetName": "optional",
    "stepId": "optional",
    "diagnostic": "optional sanitized diagnostic"
  }
}
```

This keeps failure handling machine-readable while preserving enough structure for human triage.

## Exit Semantics

P2-T8 will use a stable three-state mapping:

- `0` = `passed`
- `1` = `failed`
- `2` = `blocked`

### Semantics

- `passed`: the suite executed and satisfied the pass criteria
- `failed`: the suite executed but one or more targets/assertions failed
- `blocked`: the suite could not be executed honestly because prerequisites, policy gates, or runtime availability prevented it

`blocked` must not be collapsed into `failed`. External pipelines need to distinguish execution failure from execution impossibility.

## Redaction Policy

P2-T8 inherits the Phase 2 hardening rule that all new outputs are redacted by default. The CI handoff JSON artifact must therefore:

- mask authentication secrets such as bearer tokens, API keys, cookies, and secret-backed variables
- avoid dumping raw request/response bodies when they contain sensitive values unless safely sanitized
- expose only preview-safe strings or manifest metadata where possible
- never log or serialize raw secret material in diagnostics, handoff payloads, or helper responses

Preferred treatment:

- use stable placeholders such as `[REDACTED]`
- omit fields entirely when exposure is not justified
- preserve safe metadata such as key names, auth type, or `redacted: true` flags

## Artifact Storage Policy

The canonical JSON execution result remains filesystem-backed and should follow the established artifact path policy from the existing artifact service and app path utilities.

### Storage Rules

- payload file stays on disk, not as a SQLite blob
- any metadata persisted in SQLite should remain lightweight and consistent with existing artifact manifest patterns
- file names should be deterministic per run, for example `ci-execution-<runId>.json`
- caller-provided `outputDir` or `fileName` values must be validated for path safety and restricted to approved output roots

## File Impact Expectations

The current plan identifies these likely hotspots:

- modify `src-tauri/src/lib.rs`
- modify `src-tauri/src/services/runner_orchestration_service.rs`
- modify `src-tauri/src/services/artifact_service.rs`
- create `src-tauri/src/services/ci_handoff_service.rs` if a dedicated service boundary is justified
- create `src/services/ci-client.ts` if a distinct typed client is needed
- modify `src/types/dto.ts`
- create `tests/rust/ci_handoff_service_p2.rs`
- create `tests/frontend/ci-handoff-p2.test.ts`

Based on current repo seams, the design should also assume likely impact in:

- `src-tauri/src/repositories/runner_repository.rs`
- `src-tauri/src/contracts/commands.rs`
- `src/types/commands.ts`

## Testing Strategy

P2-T8 should be implemented with contract-first tests. The baseline must cover:

1. **happy path**
   - one suite produces a deterministic JSON artifact
   - invocation returns `passed`
   - exit code maps to `0`

2. **failed execution**
   - one suite result fails normally
   - artifact summary and failure object remain stable
   - exit code maps to `1`

3. **blocked execution**
   - one suite is prevented from running by a prerequisite or runtime gate
   - artifact still records machine-readable blocked diagnostics when possible
   - exit code maps to `2`

4. **redaction safety**
   - secret-backed inputs never leak into the JSON artifact or helper payloads

## Verification and Evidence

P2-T8 must satisfy the plan acceptance criteria:

- machine-readable run output format is documented and stable
- CI handoff path exposes clear success/fail/blocked semantics
- exported payloads remain secret-safe and artifact-consistent

Evidence files already named by the phase plan:

- `.sisyphus/evidence/p2-task-T8-ci-output.txt`
- `.sisyphus/evidence/p2-task-T8-ci-redaction.txt`

Implementation may add an extra blocked-semantics evidence file if that materially improves honesty and repeatability, but the above two are mandatory.

## Out of Scope

The following are intentionally out of scope for P2-T8:

- full CLI platform for all TestForge functionality
- remote execution control plane
- multi-suite batching in one request
- distributed worker orchestration
- retry engines, queueing layers, or scheduler replacement
- pipeline callback/webhook services
- custom external suite-definition DSLs
- schema expansion for multi-run aggregation

## Open Implementation Details

These details should be finalized during the implementation plan, but they do not change the approved design direction:

1. whether the typed invocation lands directly in `runner_orchestration_service.rs` or through a dedicated `ci_handoff_service.rs`
2. whether the thin response should include only `artifactPath` or also a stable relative path field
3. which blocked scenarios can still guarantee artifact persistence after failure normalization

## Recommendation

Proceed with a **minimal hybrid** baseline:

- one typed handoff command
- one versioned canonical JSON artifact
- one thin response DTO
- stable `0 / 1 / 2` exit mapping for `passed / failed / blocked`
- shared reuse of runner orchestration, repository read models, artifact persistence, and redaction policy

This delivers a useful CI/CD handoff contract without violating the Phase 2 guardrails.
