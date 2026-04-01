import { invokeCommand } from "./tauri-client";

export const runnerClient = {
  async executeSuite(input: { suiteId: string; environmentId: string; rerunFailedFromRunId?: string }) {
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
