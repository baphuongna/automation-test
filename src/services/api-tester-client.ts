import type { CommandError } from "./tauri-client";
import { invokeCommand } from "./tauri-client";
import { apiTesterPreviewClient } from "./api-tester-preview-client";
import type { ApiAssertionDto, ApiExecutionResultDto, ApiRequestDto, ApiTestCaseDto, CommandName } from "../types";

const WORKSPACE_STORAGE_KEY = "testforge.apiTester.workspace.v1";
export const API_TESTER_PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T9 verification path.";
export const API_TESTER_WORKSPACE_BANNER =
  "Collection tree uses a local workspace cache because the current T8 surface does not expose api.list/load commands.";

function isTauriRuntimeAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function shouldUsePreviewFallback(): boolean {
  return !isTauriRuntimeAvailable() && apiTesterPreviewClient.isAvailable();
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

async function unwrapCommand<TName extends "api.testcase.upsert" | "api.testcase.delete" | "api.execute">(
  command: TName,
  payload: Parameters<typeof invokeCommand<TName>>[1]
): Promise<Awaited<ReturnType<typeof invokeCommand<TName>>>["data"]> {
  const result = await invokeCommand(command, payload);

  if (!result.success || result.data === null) {
    throw result.error ?? createFallbackError(command);
  }

  return result.data;
}

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readWorkspaceCache(): ApiTestCaseDto[] {
  if (!canUseBrowserStorage()) {
    return [];
  }

  const raw = window.localStorage.getItem(WORKSPACE_STORAGE_KEY);
  if (!raw) {
    return [];
  }

  try {
    return JSON.parse(raw) as ApiTestCaseDto[];
  } catch {
    return [];
  }
}

function writeWorkspaceCache(testCases: ApiTestCaseDto[]): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(WORKSPACE_STORAGE_KEY, JSON.stringify(testCases));
}

function upsertWorkspaceCache(testCase: ApiTestCaseDto): void {
  const current = readWorkspaceCache();
  const next = [...current];
  const existingIndex = next.findIndex((candidate) => candidate.id === testCase.id);

  if (existingIndex >= 0) {
    next.splice(existingIndex, 1, testCase);
  } else {
    next.push(testCase);
  }

  writeWorkspaceCache(next);
}

function deleteWorkspaceCache(id: string): void {
  writeWorkspaceCache(readWorkspaceCache().filter((testCase) => testCase.id !== id));
}

export const apiTesterClient = {
  loadWorkspace(): Promise<ApiTestCaseDto[]> {
    if (shouldUsePreviewFallback()) {
      return apiTesterPreviewClient.loadWorkspace();
    }

    return Promise.resolve(readWorkspaceCache());
  },

  async upsert(testCase: ApiTestCaseDto): Promise<ApiTestCaseDto> {
    if (shouldUsePreviewFallback()) {
      return apiTesterPreviewClient.upsert(testCase);
    }

    const saved = (await unwrapCommand("api.testcase.upsert", testCase)) as ApiTestCaseDto;
    upsertWorkspaceCache(saved);
    return saved;
  },

  async delete(id: string): Promise<{ deleted: true }> {
    if (shouldUsePreviewFallback()) {
      return apiTesterPreviewClient.delete(id);
    }

    const deleted = (await unwrapCommand("api.testcase.delete", { id })) as { deleted: true };
    deleteWorkspaceCache(id);
    return deleted;
  },

  execute(input: {
    testCaseId?: string;
    environmentId: string;
    request: ApiRequestDto;
    assertions: ApiAssertionDto[];
  }): Promise<ApiExecutionResultDto> {
    if (shouldUsePreviewFallback()) {
      return apiTesterPreviewClient.execute(input);
    }

    return unwrapCommand("api.execute", input) as Promise<ApiExecutionResultDto>;
  }
};

export function getApiTesterPreviewBanner(): string | null {
  return shouldUsePreviewFallback() ? API_TESTER_PREVIEW_FALLBACK_BANNER : null;
}

export function getApiTesterWorkspaceBanner(): string | null {
  return shouldUsePreviewFallback() ? null : API_TESTER_WORKSPACE_BANNER;
}
