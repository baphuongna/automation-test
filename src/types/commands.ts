import type {
  ApiAssertionDto,
  ApiExecutionResultDto,
  ApiRequestDto,
  ApiTestCaseDto,
  BrowserHealthDto,
  DataTableDto,
  DataTableExportDto,
  DataTableImportResultDto,
  DataTableRowDto,
  EnvironmentDto,
  EnvironmentVariableDto,
  RunDetailDto,
  RunHistoryDto,
  RunHistoryEntryDto,
  CiHandoffResultDto,
  ShellMetadataDto,
  SuiteScheduleDto,
  SuiteDto,
  UiReplayResultDto,
  UiTestCaseDto
} from "./dto";
import type { EntityId, EnvironmentType, IsoDateTime, RunStatus } from "./domain";

export interface CommandPayloadMap {
  "environment.list": Record<string, never>;
  "environment.create": {
    name: string;
    envType: EnvironmentType;
    isDefault: boolean;
  };
  "environment.update": {
    id: EntityId;
    name: string;
    envType: EnvironmentType;
    isDefault: boolean;
  };
  "environment.delete": {
    id: EntityId;
  };
  "environment.variable.upsert": {
    environmentId: EntityId;
    variable: Pick<EnvironmentVariableDto, "id" | "key" | "kind"> & { value: string };
  };
  "environment.variable.delete": {
    id: EntityId;
  };
  "dataTable.list": Record<string, never>;
  "dataTable.create": {
    name: string;
    description?: string;
    columns: DataTableDto["columns"];
  };
  "dataTable.update": {
    id: EntityId;
    name: string;
    description?: string;
    columns: DataTableDto["columns"];
  };
  "dataTable.delete": {
    id: EntityId;
  };
  "dataTable.row.upsert": {
    tableId: EntityId;
    row: Pick<DataTableRowDto, "id" | "values" | "enabled" | "rowIndex">;
  };
  "dataTable.row.delete": {
    id: EntityId;
  };
  "dataTable.import": {
    tableId?: EntityId;
    name: string;
    description?: string;
    format: "csv" | "json";
    content: string;
  };
  "dataTable.export": {
    id: EntityId;
    format: "csv" | "json";
  };
  "api.testcase.upsert": ApiTestCaseDto;
  "api.testcase.delete": {
    id: EntityId;
  };
  "api.execute": {
    testCaseId?: EntityId;
    environmentId: EntityId;
    request: ApiRequestDto;
    assertions: ApiAssertionDto[];
  };
  "ui.testcase.upsert": UiTestCaseDto;
  "ui.testcase.get": {
    id: EntityId;
  };
  "ui.testcase.delete": {
    id: EntityId;
  };
  "browser.recording.start": {
    testCaseId: EntityId;
    startUrl: string;
  };
  "shell.metadata.get": Record<string, never>;
  "browser.health.check": Record<string, never>;
  "browser.recording.stop": {
    testCaseId: EntityId;
  };
  "browser.recording.cancel": {
    testCaseId: EntityId;
  };
  "browser.replay.start": {
    testCaseId: EntityId;
    environmentId?: EntityId;
  };
  "browser.replay.cancel": {
    runId: EntityId;
  };
  "runner.suite.execute": {
    suiteId: EntityId;
    environmentId: EntityId;
    rerunFailedFromRunId?: EntityId;
  };
  "runner.suite.list": Record<string, never>;
  "runner.run.history": {
    suiteId?: EntityId;
    status?: Exclude<RunStatus, "idle">;
    startedAfter?: IsoDateTime;
    startedBefore?: IsoDateTime;
  };
  "runner.run.detail": {
    runId: EntityId;
  };
  "runner.suite.cancel": {
    runId: EntityId;
  };
  "scheduler.schedule.list": Record<string, never>;
  "scheduler.schedule.upsert": {
    scheduleId?: EntityId;
    suiteId: EntityId;
    environmentId: EntityId;
    cadenceMinutes: number;
    enabled: boolean;
  };
  "scheduler.schedule.setEnabled": {
    scheduleId: EntityId;
    enabled: boolean;
  };
  "scheduler.schedule.delete": {
    scheduleId: EntityId;
  };
  "ci.handoff.execute": {
    suiteId: EntityId;
    trigger: {
      source: "ci";
      actor: "pipeline";
      label?: string;
    };
    output: {
      writeJson: true;
      outputDir?: string;
      fileName?: string;
    };
  };
}

export interface CommandResponseMap {
  "environment.list": EnvironmentDto[];
  "environment.create": EnvironmentDto;
  "environment.update": EnvironmentDto;
  "environment.delete": { deleted: true };
  "environment.variable.upsert": EnvironmentVariableDto;
  "environment.variable.delete": { deleted: true };
  "dataTable.list": DataTableDto[];
  "dataTable.create": DataTableDto;
  "dataTable.update": DataTableDto;
  "dataTable.delete": { deleted: true };
  "dataTable.row.upsert": DataTableRowDto;
  "dataTable.row.delete": { deleted: true };
  "dataTable.import": DataTableImportResultDto;
  "dataTable.export": DataTableExportDto;
  "api.testcase.upsert": ApiTestCaseDto;
  "api.testcase.delete": { deleted: true };
  "api.execute": ApiExecutionResultDto;
  "ui.testcase.upsert": UiTestCaseDto;
  "ui.testcase.get": UiTestCaseDto;
  "ui.testcase.delete": { deleted: true };
  "browser.recording.start": { started: true };
  "shell.metadata.get": ShellMetadataDto;
  "browser.health.check": BrowserHealthDto;
  "browser.recording.stop": UiTestCaseDto;
  "browser.recording.cancel": { cancelled: true };
  "browser.replay.start": UiReplayResultDto;
  "browser.replay.cancel": { cancelled: true };
  "runner.suite.execute": {
    runId: EntityId;
    suite: SuiteDto;
  };
  "runner.suite.list": SuiteDto[];
  "runner.run.history": RunHistoryDto;
  "runner.run.detail": RunDetailDto;
  "runner.suite.cancel": { cancelled: true };
  "scheduler.schedule.list": SuiteScheduleDto[];
  "scheduler.schedule.upsert": SuiteScheduleDto;
  "scheduler.schedule.setEnabled": SuiteScheduleDto;
  "scheduler.schedule.delete": { deleted: true };
  "ci.handoff.execute": CiHandoffResultDto;
}

export type CommandName = keyof CommandPayloadMap;

export type CommandEnvelope<TName extends CommandName = CommandName> = {
  command: TName;
  payload: CommandPayloadMap[TName];
};
