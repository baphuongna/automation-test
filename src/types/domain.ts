export type EntityId = string;
export type IsoDateTime = string;

export const TEST_CASE_TYPES = ["api", "ui"] as const;
export type TestCaseType = (typeof TEST_CASE_TYPES)[number];

export const RUN_STATUSES = [
  "idle",
  "queued",
  "running",
  "passed",
  "failed",
  "cancelled"
] as const;
export type RunStatus = (typeof RUN_STATUSES)[number];

export const BROWSER_RUNTIME_STATUSES = ["healthy", "degraded", "unavailable"] as const;
export type BrowserRuntimeStatus = (typeof BROWSER_RUNTIME_STATUSES)[number];

export const RECORDING_STATUSES = ["idle", "recording", "stopped", "failed"] as const;
export type RecordingStatus = (typeof RECORDING_STATUSES)[number];

export const REPLAY_STATUSES = ["idle", "running", "passed", "failed", "cancelled"] as const;
export type ReplayStatus = (typeof REPLAY_STATUSES)[number];

export const VARIABLE_KINDS = ["plain", "secret"] as const;
export type VariableKind = (typeof VARIABLE_KINDS)[number];

export const ASSERTION_OPERATORS = [
  "status_equals",
  "json_path_exists",
  "json_path_equals",
  "body_contains",
  "header_equals"
] as const;
export type AssertionOperator = (typeof ASSERTION_OPERATORS)[number];

export const STEP_ACTIONS = [
  "navigate",
  "click",
  "fill",
  "select",
  "check",
  "uncheck",
  "wait_for",
  "assert_text"
] as const;
export type StepAction = (typeof STEP_ACTIONS)[number];
