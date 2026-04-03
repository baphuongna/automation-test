import type { SuiteScheduleDto } from "../types";
import { invokeCommand } from "./tauri-client";

export const schedulerClient = {
  async listSchedules() {
    const result = await invokeCommand("scheduler.schedule.list", {});
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for scheduler.schedule.list");
    }

    return result.data;
  },

  async upsertSchedule(input: {
    scheduleId?: string;
    suiteId: string;
    environmentId: string;
    cadenceMinutes: number;
    enabled: boolean;
  }): Promise<SuiteScheduleDto> {
    const result = await invokeCommand("scheduler.schedule.upsert", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for scheduler.schedule.upsert");
    }

    return result.data;
  },

  async setScheduleEnabled(input: { scheduleId: string; enabled: boolean }): Promise<SuiteScheduleDto> {
    const result = await invokeCommand("scheduler.schedule.setEnabled", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for scheduler.schedule.setEnabled");
    }

    return result.data;
  },

  async deleteSchedule(input: { scheduleId: string }): Promise<{ deleted: true }> {
    const result = await invokeCommand("scheduler.schedule.delete", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for scheduler.schedule.delete");
    }

    return result.data;
  }
};
