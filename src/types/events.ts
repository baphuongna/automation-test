import type { ErrorPayload } from "./errors";
import type { BrowserHealthDto, RunResultDto, UiStepDto } from "./dto";
import type { EntityId, RecordingStatus, ReplayStatus, RunStatus, TestCaseType } from "./domain";

export interface EventPayloadMap {
  "app.error": {
    scope: "global" | "command" | "runner";
    error: ErrorPayload;
  };
  "browser.health.changed": BrowserHealthDto;
  "browser.recording.status.changed": {
    testCaseId: EntityId;
    status: RecordingStatus;
  };
  "browser.recording.step.captured": {
    testCaseId: EntityId;
    step: UiStepDto;
  };
  "browser.replay.progress": {
    runId: EntityId;
    status: ReplayStatus;
    currentStepId?: EntityId;
  };
  "runner.execution.started": {
    runId: EntityId;
    suiteId: EntityId;
  };
  "runner.execution.progress": {
    runId: EntityId;
    testCaseId: EntityId;
    testCaseType: TestCaseType;
    status: RunStatus;
  };
  "runner.execution.completed": RunResultDto;
}

export type EventName = keyof EventPayloadMap;

export type EventEnvelope<TName extends EventName = EventName> = {
  event: TName;
  payload: EventPayloadMap[TName];
};
