import { invokeCommand } from "./tauri-client";
import type { RunHistoryDto, RunHistoryEntryDto } from "../types";

type RunHistoryStatusFilter = Exclude<RunHistoryEntryDto["status"], "idle">;

export const runnerClient = {
  async listSuites() {
    const result = await invokeCommand("runner.suite.list", {});
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.suite.list");
    }

    return result.data;
  },

  async listRunHistory(input: {
    suiteId?: string;
    status?: RunHistoryStatusFilter;
    startedAfter?: string;
    startedBefore?: string;
  } = {}) {
    const payload = {
      ...(input.suiteId ? { suiteId: input.suiteId } : {}),
      ...(input.status ? { status: input.status } : {}),
      ...(input.startedAfter ? { startedAfter: input.startedAfter } : {}),
      ...(input.startedBefore ? { startedBefore: input.startedBefore } : {})
    };
    const result = await invokeCommand("runner.run.history", payload);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.run.history");
    }

    return result.data.entries;
  },

  async listRunHistoryReport(input: {
    suiteId?: string;
    status?: RunHistoryStatusFilter;
    startedAfter?: string;
    startedBefore?: string;
  } = {}): Promise<RunHistoryDto> {
    const payload = {
      ...(input.suiteId ? { suiteId: input.suiteId } : {}),
      ...(input.status ? { status: input.status } : {}),
      ...(input.startedAfter ? { startedAfter: input.startedAfter } : {}),
      ...(input.startedBefore ? { startedBefore: input.startedBefore } : {})
    };
    const result = await invokeCommand("runner.run.history", payload);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.run.history");
    }

    return result.data;
  },

  async getRunDetail(input: { runId: string }) {
    const result = await invokeCommand("runner.run.detail", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.run.detail");
    }

    return result.data;
  },

  async executeSuite(input: { suiteId: string; environmentId: string } | { suiteId: string; environmentId: string; rerunFailedFromRunId: string }) {
    const result = await invokeCommand("runner.suite.execute", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.suite.execute");
    }

    return result.data;
  },

  async cancelSuite(input: { runId: string }) {
    const result = await invokeCommand("runner.suite.cancel", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for runner.suite.cancel");
    }

    return result.data;
  }
};
