import { invokeCommand } from "./tauri-client";
import type { CommandPayloadMap, CiHandoffResultDto } from "../types";

export const ciClient = {
  async executeCiHandoff(input: CommandPayloadMap["ci.handoff.execute"]): Promise<CiHandoffResultDto> {
    const result = await invokeCommand("ci.handoff.execute", input);
    if (!result.success || result.data === null) {
      throw result.error ?? new Error("Missing command result for ci.handoff.execute");
    }

    return result.data;
  }
};
