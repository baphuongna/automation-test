import { invoke } from "@tauri-apps/api/core";
import type { CommandName, CommandPayloadMap, CommandResponseMap, ErrorPayload } from "../types";

export interface CommandError extends ErrorPayload {}

const FALLBACK_COMMAND_ERROR_CODE: ErrorPayload['code'] = 'RUNNER_EXECUTION_FAILED';

export interface CommandResult<T> {
  data: T | null;
  error: CommandError | null;
  success: boolean;
}

function toCommandError(error: unknown, command: CommandName): CommandError {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<CommandError>;

    if (typeof candidate.code === "string" && typeof candidate.displayMessage === "string") {
      return {
        code: candidate.code,
        context: candidate.context ?? {},
        displayMessage: candidate.displayMessage,
        recoverable: candidate.recoverable ?? false,
        technicalMessage:
          typeof candidate.technicalMessage === "string"
            ? candidate.technicalMessage
            : candidate.displayMessage
      };
    }
  }

  const technicalMessage = error instanceof Error ? error.message : String(error);

  return {
    code: FALLBACK_COMMAND_ERROR_CODE,
    context: { command },
    displayMessage: `Không thể thực hiện lệnh ${command}.`,
    recoverable: false,
    technicalMessage
  };
}

export async function invokeCommand<TName extends CommandName>(
  command: TName,
  payload: CommandPayloadMap[TName]
): Promise<CommandResult<CommandResponseMap[TName]>> {
  try {
    const data = await invoke<CommandResponseMap[TName]>(command, payload as Record<string, unknown>);

    return {
      data,
      error: null,
      success: true
    };
  } catch (error) {
    return {
      data: null,
      error: toCommandError(error, command),
      success: false
    };
  }
}
