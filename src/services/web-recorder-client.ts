import type { CommandError } from "./tauri-client";
import { invokeCommand } from "./tauri-client";
import { webRecorderPreviewClient } from "./web-recorder-preview-client";
import type {
  BrowserHealthDto,
  CommandName,
  CommandPayloadMap,
  CommandResponseMap,
  UiStepDto,
  UiTestCaseDto
} from "../types";

export const WEB_RECORDER_PREVIEW_FALLBACK_BANNER =
  "Preview fallback active. Desktop runtime capability checks are unavailable in this browser-only mode.";
export const WEB_RECORDER_WORKSPACE_BANNER =
  "Recorder draft prefers persisted desktop storage when available and falls back to the local workspace cache. Replay is a desktop runtime capability and stays gated by preflight health.";

function isTauriRuntimeAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function shouldUsePreviewFallback(): boolean {
  return !isTauriRuntimeAvailable() && webRecorderPreviewClient.isAvailable();
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

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

const WORKSPACE_STORAGE_KEY = "testforge.webRecorder.workspace.v1";
const DEFAULT_DRAFT_ID = "ui-recorder-draft";

function createDefaultDraft(): UiTestCaseDto {
  return {
    id: DEFAULT_DRAFT_ID,
    type: "ui",
    name: "Untitled recorder draft",
    startUrl: "",
    steps: []
  };
}

function readWorkspaceCache(): UiTestCaseDto {
  if (!canUseBrowserStorage()) {
    return createDefaultDraft();
  }

  const raw = window.localStorage.getItem(WORKSPACE_STORAGE_KEY);
  if (!raw) {
    return createDefaultDraft();
  }

  try {
    return JSON.parse(raw) as UiTestCaseDto;
  } catch {
    return createDefaultDraft();
  }
}

function writeWorkspaceCache(testCase: UiTestCaseDto): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(WORKSPACE_STORAGE_KEY, JSON.stringify(testCase));
}

function isPersistedDraftId(id: string): boolean {
  return id.trim().length > 0 && id !== DEFAULT_DRAFT_ID;
}

async function getById(id: string): Promise<UiTestCaseDto> {
  return unwrapCommand("ui.testcase.get", { id });
}

async function hydratePersistedWorkspace(cachedDraft: UiTestCaseDto): Promise<UiTestCaseDto> {
  if (!isPersistedDraftId(cachedDraft.id)) {
    return cachedDraft;
  }

  try {
    const persistedDraft = await getById(cachedDraft.id);
    writeWorkspaceCache(persistedDraft);
    return persistedDraft;
  } catch {
    return cachedDraft;
  }
}

function nextStepId(): string {
  return `ui-step-${Math.random().toString(36).slice(2, 10)}`;
}

function normalizeStep(step: UiStepDto): UiStepDto {
  const selector = step.selector?.trim();
  const value = step.value?.trim();
  const timeoutMs = step.timeoutMs ?? 5000;

  return {
    id: step.id.trim().length > 0 ? step.id : nextStepId(),
    action: step.action,
    ...(selector ? { selector } : {}),
    ...(value ? { value } : {}),
    timeoutMs,
    ...(step.confidence ? { confidence: step.confidence } : {})
  };
}

function normalizeDraft(testCase: UiTestCaseDto): UiTestCaseDto {
  return {
    id: testCase.id.trim().length > 0 ? testCase.id.trim() : createDefaultDraft().id,
    type: "ui",
    name: testCase.name.trim().length > 0 ? testCase.name.trim() : "Untitled recorder draft",
    startUrl: testCase.startUrl.trim(),
    steps: testCase.steps.map(normalizeStep)
  };
}

export const webRecorderClient = {
  async loadWorkspace(): Promise<UiTestCaseDto> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.loadWorkspace();
    }

    const cachedDraft = readWorkspaceCache();
    return hydratePersistedWorkspace(cachedDraft);
  },

  checkHealth(): Promise<BrowserHealthDto> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.checkHealth();
    }

    return unwrapCommand("browser.health.check", {});
  },

  async upsert(testCase: UiTestCaseDto): Promise<UiTestCaseDto> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.upsert(testCase);
    }

    const normalized = normalizeDraft(testCase);
    const saved = await unwrapCommand("ui.testcase.upsert", normalized);
    writeWorkspaceCache(saved);
    return saved;
  },

  async delete(id: string): Promise<{ deleted: true }> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.remove(id);
    }

    const deleted = await unwrapCommand("ui.testcase.delete", { id });
    writeWorkspaceCache(createDefaultDraft());
    return deleted;
  },

  startRecording(input: { testCaseId: string; startUrl: string }): Promise<{ started: true }> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.startRecording(input);
    }

    return unwrapCommand("browser.recording.start", input);
  },

  async stopRecording(input: { testCaseId: string }): Promise<UiTestCaseDto> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.stopRecording(input);
    }

    const saved = await unwrapCommand("browser.recording.stop", input);
    writeWorkspaceCache(saved);
    return saved;
  },

  cancelRecording(input: { testCaseId: string }): Promise<{ cancelled: true }> {
    if (shouldUsePreviewFallback()) {
      return webRecorderPreviewClient.cancelRecording(input);
    }

    return unwrapCommand("browser.recording.cancel", input);
  }
};

export function getWebRecorderPreviewBanner(): string | null {
  return shouldUsePreviewFallback() ? WEB_RECORDER_PREVIEW_FALLBACK_BANNER : null;
}

export function getWebRecorderWorkspaceBanner(): string | null {
  return shouldUsePreviewFallback() ? null : WEB_RECORDER_WORKSPACE_BANNER;
}
