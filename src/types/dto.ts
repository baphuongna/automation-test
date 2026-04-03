import type {
  AssertionOperator,
  EntityId,
  EnvironmentType,
  IsoDateTime,
  ReplayStatus,
  RunStatus,
  StepConfidence,
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
  envType: EnvironmentType;
  isDefault: boolean;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  variables: EnvironmentVariableDto[];
}

export interface DataTableColumnDto {
  name: string;
  colType: string;
}

export interface DataTableRowDto {
  id: EntityId;
  values: string[];
  enabled: boolean;
  rowIndex: number;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface DataTableAssociationMetadataDto {
  canAssociateToTestCases: boolean;
  linkedTestCaseIds: EntityId[];
  totalRowCount: number;
  enabledRowCount: number;
}

export interface DataTableDto {
  id: EntityId;
  name: string;
  description?: string;
  columns: DataTableColumnDto[];
  rows: DataTableRowDto[];
  associationMeta: DataTableAssociationMetadataDto;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface DataTableImportResultDto {
  table: DataTableDto;
  importedRowCount: number;
  format: "csv" | "json";
}

export interface DataTableExportDto {
  fileName: string;
  format: "csv" | "json";
  content: string;
  table: DataTableDto;
}

export interface ArtifactManifestDto {
  id: EntityId;
  artifactType: string;
  logicalName: string;
  filePath: string;
  relativePath: string;
  previewJson: string;
  createdAt: IsoDateTime;
}

export interface ReportExportDto {
  fileName: string;
  format: "html" | "json";
  filePath: string;
  manifest: ArtifactManifestDto;
}

export interface ApiAssertionDto {
  id: EntityId;
  operator: AssertionOperator;
  expectedValue: string;
  sourcePath?: string;
}

export interface ApiAuthDto {
  type: "none" | "bearer" | "basic" | "api_key";
  location?: "header" | "query";
  key?: string;
  value?: string;
  token?: string;
  username?: string;
  password?: string;
}

export interface ApiRequestDto {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  url: string;
  headers: Record<string, string>;
  queryParams: Record<string, string>;
  body?: string;
  auth?: ApiAuthDto;
}

export interface ApiAssertionResultDto {
  assertionId: EntityId;
  operator: AssertionOperator;
  passed: boolean;
  expectedValue: string;
  actualValue?: string;
  sourcePath?: string;
  errorCode?: string;
  message?: string;
}

export interface ApiRequestPreviewDto {
  method: string;
  url: string;
  headers: Record<string, string>;
  queryParams: Record<string, string>;
  bodyPreview?: string;
  authPreview: string;
}

export interface ApiExecutionResultDto {
  status: "passed" | "failed";
  transportSuccess: boolean;
  failureKind?: "preflight" | "transport" | "assertion";
  errorCode?: string;
  errorMessage?: string;
  statusCode?: number;
  durationMs: number;
  bodyPreview: string;
  responseHeaders: Record<string, string>;
  assertions: ApiAssertionResultDto[];
  requestPreview: ApiRequestPreviewDto;
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
  confidence?: StepConfidence;
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

export interface SuiteScheduleDto {
  id: EntityId;
  suiteId: EntityId;
  environmentId: EntityId;
  enabled: boolean;
  cadenceMinutes: number;
  lastRunAt?: IsoDateTime;
  nextRunAt?: IsoDateTime;
  lastRunStatus?: Exclude<RunStatus, "idle">;
  lastError?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface RunResultDto {
  runId: EntityId;
  status: RunStatus;
  suiteId?: EntityId;
  environmentId?: EntityId;
  startedAt: IsoDateTime;
  finishedAt?: IsoDateTime;
  totalCount: number;
  passedCount: number;
  failedCount: number;
  skippedCount: number;
}

export interface RunHistoryEntryDto extends RunResultDto {
  suiteName?: string;
  environmentName: string;
}

export interface RunHistoryFilterDto {
  suiteId?: EntityId;
  status?: Exclude<RunStatus, "idle">;
  startedAfter?: IsoDateTime;
  startedBefore?: IsoDateTime;
}

export interface RunHistoryGroupSummaryDto {
  totalRuns: number;
  passedRuns: number;
  failedRuns: number;
  cancelledRuns: number;
  failureCategoryCounts: Array<{
    category: string;
    count: number;
  }>;
}

export interface RunHistoryDto {
  entries: RunHistoryEntryDto[];
  groupSummary: RunHistoryGroupSummaryDto;
}

export interface RunCaseResultDto {
  id: EntityId;
  caseId: EntityId;
  caseName: string;
  testCaseType: TestCaseType;
  dataRowId?: EntityId;
  dataRowLabel?: string;
  status: RunStatus;
  durationMs: number;
  errorMessage?: string;
  errorCode?: string;
  failureCategory: string;
  requestPreview: string;
  responsePreview: string;
  assertionPreview: string;
  artifacts: ArtifactManifestDto[];
}

export interface RunDetailDto {
  summary: RunHistoryEntryDto;
  results: RunCaseResultDto[];
  artifacts: ArtifactManifestDto[];
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

export interface ShellMetadataDto {
  appVersion: string;
  isFirstRun: boolean;
  degradedMode: boolean;
  masterKeyInitialized: boolean;
  browserRuntime: BrowserHealthDto;
}
