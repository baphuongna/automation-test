import type {
  AssertionOperator,
  EntityId,
  IsoDateTime,
  ReplayStatus,
  RunStatus,
  StepAction,
  TestCaseType,
  VariableKind
} from "./domain";

export interface EnvironmentVariableDto {
  id: EntityId;
  key: string;
  kind: VariableKind;
  valueMaskedPreview: string;
}

export interface EnvironmentDto {
  id: EntityId;
  name: string;
  isDefault: boolean;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  variables: EnvironmentVariableDto[];
}

export interface ApiAssertionDto {
  id: EntityId;
  operator: AssertionOperator;
  expectedValue: string;
  sourcePath?: string;
}

export interface ApiRequestDto {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  url: string;
  headers: Record<string, string>;
  queryParams: Record<string, string>;
  body?: string;
}

export interface ApiTestCaseDto {
  id: EntityId;
  type: Extract<TestCaseType, "api">;
  name: string;
  request: ApiRequestDto;
  assertions: ApiAssertionDto[];
}

export interface UiStepDto {
  id: EntityId;
  action: StepAction;
  selector?: string;
  value?: string;
  timeoutMs?: number;
}

export interface UiTestCaseDto {
  id: EntityId;
  type: Extract<TestCaseType, "ui">;
  name: string;
  startUrl: string;
  steps: UiStepDto[];
}

export interface SuiteItemDto {
  id: EntityId;
  testCaseId: EntityId;
  type: TestCaseType;
  order: number;
}

export interface SuiteDto {
  id: EntityId;
  name: string;
  items: SuiteItemDto[];
}

export interface RunResultDto {
  runId: EntityId;
  status: RunStatus;
  startedAt: IsoDateTime;
  finishedAt?: IsoDateTime;
  passedCount: number;
  failedCount: number;
}

export interface UiReplayResultDto {
  runId: EntityId;
  status: ReplayStatus;
  failedStepId?: EntityId;
  screenshotPath?: string;
}

export interface BrowserHealthDto {
  runtimeStatus: import("./domain").BrowserRuntimeStatus;
  message: string;
  checkedAt: IsoDateTime;
}
