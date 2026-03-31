import type { CommandError } from "./tauri-client";
import { invokeCommand } from "./tauri-client";
import { environmentPreviewClient } from "./environment-preview-client";
import type { CommandName, CommandPayloadMap, CommandResponseMap, EnvironmentDto } from "../types";

export const PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T6 verification path.";

function isTauriRuntimeAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function shouldUsePreviewFallback(): boolean {
  return !isTauriRuntimeAvailable() && environmentPreviewClient.isAvailable();
}

async function unwrapCommand<TName extends CommandName>(
  command: TName,
  payload: CommandPayloadMap[TName]
): Promise<CommandResponseMap[TName]> {
  const result = await invokeCommand(command, payload);

  if (!result.success || result.data === null) {
    throw result.error ?? createFallbackError(command);
  }

  return result.data;
}

function createFallbackError(command: CommandName): CommandError {
  return {
    code: "INTERNAL_UNEXPECTED_ERROR",
    context: { command },
    displayMessage: `Không thể thực hiện lệnh ${command}.`,
    recoverable: false,
    technicalMessage: `Missing command result for ${command}`
  };
}

export interface EnvironmentSaveInput {
  id?: string;
  name: string;
  envType: EnvironmentDto["envType"];
  isDefault: boolean;
}

export interface EnvironmentVariableSaveInput {
  environmentId: string;
  id?: string;
  key: string;
  kind: "plain" | "secret";
  value: string;
}

export const environmentClient = {
  list(): Promise<EnvironmentDto[]> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.list();
    }

    return unwrapCommand("environment.list", {});
  },

  create(input: EnvironmentSaveInput): Promise<EnvironmentDto> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.create(input);
    }

    return unwrapCommand("environment.create", {
      envType: input.envType,
      isDefault: input.isDefault,
      name: input.name
    });
  },

  update(input: EnvironmentSaveInput & { id: string }): Promise<EnvironmentDto> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.update(input);
    }

    return unwrapCommand("environment.update", {
      envType: input.envType,
      id: input.id,
      isDefault: input.isDefault,
      name: input.name
    });
  },

  remove(id: string): Promise<{ deleted: true }> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.remove(id);
    }

    return unwrapCommand("environment.delete", { id });
  },

  upsertVariable(input: EnvironmentVariableSaveInput): Promise<CommandResponseMap["environment.variable.upsert"]> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.upsertVariable(input);
    }

    return unwrapCommand("environment.variable.upsert", {
      environmentId: input.environmentId,
      variable: {
        id: input.id ?? "",
        key: input.key,
        kind: input.kind,
        value: input.value
      }
    });
  },

  deleteVariable(id: string): Promise<{ deleted: true }> {
    if (shouldUsePreviewFallback()) {
      return environmentPreviewClient.deleteVariable(id);
    }

    return unwrapCommand("environment.variable.delete", { id });
  }
};

export function getEnvironmentPreviewBanner(): string | null {
  return shouldUsePreviewFallback() ? PREVIEW_FALLBACK_BANNER : null;
}

export function isEnvironmentPreviewDegradedMode(): boolean {
  return shouldUsePreviewFallback() && environmentPreviewClient.isDegraded();
}

export function isSecretStoreBlockedError(error: CommandError | null | undefined): boolean {
  if (!error) {
    return false;
  }

  return ["SECRET_KEY_MISSING", "SECURITY_KEY_CORRUPTED", "SECURITY_SECRET_ACCESS_DENIED"].includes(
    error.code
  );
}
