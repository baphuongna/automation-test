import type {
  ApiRequestDto,
  ApiTestCaseDto,
  EnvironmentDto,
  EnvironmentVariableDto,
  SuiteDto,
  UiTestCaseDto
} from "./dto";
import type { EntityId } from "./domain";

export interface CommandPayloadMap {
  "environment.list": Record<string, never>;
  "environment.create": {
    name: string;
  };
  "environment.update": {
    id: EntityId;
    name: string;
    isDefault: boolean;
  };
  "environment.delete": {
    id: EntityId;
  };
  "environment.variable.upsert": {
    environmentId: EntityId;
    variable: Pick<EnvironmentVariableDto, "id" | "key" | "kind"> & { value: string };
  };
  "api.testcase.upsert": ApiTestCaseDto;
  "api.testcase.delete": {
    id: EntityId;
  };
  "api.execute": {
    environmentId: EntityId;
    request: ApiRequestDto;
  };
  "ui.testcase.upsert": UiTestCaseDto;
  "ui.testcase.delete": {
    id: EntityId;
  };
  "browser.recording.start": {
    testCaseId: EntityId;
    startUrl: string;
  };
  "browser.recording.stop": {
    testCaseId: EntityId;
  };
  "browser.replay.start": {
    testCaseId: EntityId;
    environmentId?: EntityId;
  };
  "runner.suite.execute": {
    suiteId: EntityId;
    environmentId: EntityId;
  };
  "runner.suite.cancel": {
    runId: EntityId;
  };
}

export interface CommandResponseMap {
  "environment.list": EnvironmentDto[];
  "environment.create": EnvironmentDto;
  "environment.update": EnvironmentDto;
  "environment.delete": { deleted: true };
  "environment.variable.upsert": EnvironmentVariableDto;
  "api.testcase.upsert": ApiTestCaseDto;
  "api.testcase.delete": { deleted: true };
  "api.execute": {
    statusCode: number;
    durationMs: number;
    bodyPreview: string;
  };
  "ui.testcase.upsert": UiTestCaseDto;
  "ui.testcase.delete": { deleted: true };
  "browser.recording.start": { started: true };
  "browser.recording.stop": UiTestCaseDto;
  "browser.replay.start": {
    runId: EntityId;
  };
  "runner.suite.execute": {
    runId: EntityId;
    suite: SuiteDto;
  };
  "runner.suite.cancel": { cancelled: true };
}

export type CommandName = keyof CommandPayloadMap;

export type CommandEnvelope<TName extends CommandName = CommandName> = {
  command: TName;
  payload: CommandPayloadMap[TName];
};
