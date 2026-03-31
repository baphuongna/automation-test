import type { BrowserHealthDto, RecordingStatus, UiStepDto, UiTestCaseDto } from "../types";

const PREVIEW_STORAGE_KEY = "testforge.webRecorder.preview.v1";
const PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T13 verification path.";

interface WebRecorderPreviewState {
  draft: UiTestCaseDto;
  health: BrowserHealthDto;
  recordingStatus: RecordingStatus;
}

function nowIso(): string {
  return new Date().toISOString();
}

function createId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function createSeedSteps(): UiStepDto[] {
  return [
    {
      id: "ui-step-preview-navigate",
      action: "navigate",
      value: "https://preview.testforge.local/login",
      timeoutMs: 5000,
      confidence: "high"
    },
    {
      id: "ui-step-preview-fill-email",
      action: "fill",
      selector: "input[name=email]",
      value: "qa@testforge.local",
      timeoutMs: 5000,
      confidence: "high"
    },
    {
      id: "ui-step-preview-click-submit",
      action: "click",
      selector: "div.form-actions > button:nth-child(1)",
      timeoutMs: 5000,
      confidence: "low"
    }
  ];
}

function createSeedState(): WebRecorderPreviewState {
  return {
    draft: {
      id: "ui-preview-checkout",
      type: "ui",
      name: "Preview · Login flow",
      startUrl: "https://preview.testforge.local/login",
      steps: createSeedSteps()
    },
    health: {
      runtimeStatus: "healthy",
      message: "Preview browser runtime is simulated and ready.",
      checkedAt: nowIso()
    },
    recordingStatus: "idle"
  };
}

function readStoredState(): WebRecorderPreviewState {
  if (!canUseBrowserStorage()) {
    return createSeedState();
  }

  const raw = window.localStorage.getItem(PREVIEW_STORAGE_KEY);
  if (!raw) {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }

  try {
    return JSON.parse(raw) as WebRecorderPreviewState;
  } catch {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }
}

function writeStoredState(state: WebRecorderPreviewState): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(state));
}

function normalizeStep(step: UiStepDto, index: number): UiStepDto {
  const selector = step.selector?.trim();
  const value = step.value?.trim();
  const timeoutMs = step.timeoutMs ?? 5000;

  return {
    id: step.id.trim().length > 0 ? step.id : createId(`ui-step-${index + 1}`),
    action: step.action,
    ...(selector ? { selector } : {}),
    ...(value ? { value } : {}),
    timeoutMs,
    ...(step.confidence ? { confidence: step.confidence } : {})
  };
}

function normalizeDraft(testCase: UiTestCaseDto): UiTestCaseDto {
  return {
    id: testCase.id.trim().length > 0 ? testCase.id : createId("ui-preview"),
    type: "ui",
    name: testCase.name.trim().length > 0 ? testCase.name.trim() : "Untitled web recorder draft",
    startUrl: testCase.startUrl.trim(),
    steps: testCase.steps.map(normalizeStep)
  };
}

function dispatchEvent<TDetail>(eventName: string, detail: TDetail): void {
  if (typeof window === "undefined") {
    return;
  }

  window.dispatchEvent(new CustomEvent(eventName, { detail }));
}

function appendPreviewStep(draft: UiTestCaseDto, step: UiStepDto): UiTestCaseDto {
  return {
    ...draft,
    steps: [...draft.steps, normalizeStep(step, draft.steps.length)]
  };
}

export const webRecorderPreviewClient = {
  banner: PREVIEW_FALLBACK_BANNER,

  isAvailable(): boolean {
    return typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
  },

  loadWorkspace(): Promise<UiTestCaseDto> {
    return Promise.resolve(readStoredState().draft);
  },

  checkHealth(): Promise<BrowserHealthDto> {
    const state = readStoredState();
    const health = {
      ...state.health,
      checkedAt: nowIso()
    };
    writeStoredState({ ...state, health });
    dispatchEvent("browser.health.changed", health);
    return Promise.resolve(health);
  },

  async upsert(testCase: UiTestCaseDto): Promise<UiTestCaseDto> {
    const state = readStoredState();
    const draft = normalizeDraft(testCase);
    writeStoredState({ ...state, draft });
    return draft;
  },

  async remove(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    if (state.draft.id === id) {
      writeStoredState({ ...state, draft: createSeedState().draft });
    }

    return { deleted: true };
  },

  async startRecording(input: { testCaseId: string; startUrl: string }): Promise<{ started: true }> {
    const state = readStoredState();
    const navigateStep: UiStepDto = {
      id: createId("ui-step-record"),
      action: "navigate",
      value: input.startUrl.trim(),
      timeoutMs: 5000,
      confidence: "high"
    };
    const capturedStep: UiStepDto = {
      id: createId("ui-step-record"),
      action: "click",
      selector: "button[data-testid=preview-submit]",
      timeoutMs: 5000,
      confidence: "medium"
    };
    const nextState: WebRecorderPreviewState = {
      ...state,
      draft: {
        ...state.draft,
        id: input.testCaseId,
        startUrl: input.startUrl.trim(),
        steps: [normalizeStep(navigateStep, 0)]
      },
      recordingStatus: "recording"
    };
    const finalState = {
      ...nextState,
      draft: appendPreviewStep(nextState.draft, capturedStep)
    };
    writeStoredState(finalState);

    dispatchEvent("browser.recording.status.changed", {
      testCaseId: input.testCaseId,
      status: "recording"
    });

    dispatchEvent("browser.recording.step.captured", {
      testCaseId: input.testCaseId,
      step: capturedStep
    });

    return { started: true };
  },

  async stopRecording(input: { testCaseId: string }): Promise<UiTestCaseDto> {
    const state = readStoredState();
    const steps = state.draft.steps.length > 0 ? state.draft.steps : createSeedSteps();
    const draft: UiTestCaseDto = {
      ...state.draft,
      id: input.testCaseId,
      steps: steps.map(normalizeStep)
    };

    writeStoredState({
      ...state,
      draft,
      recordingStatus: "stopped"
    });

    dispatchEvent("browser.recording.status.changed", {
      testCaseId: input.testCaseId,
      status: "stopped"
    });

    return draft;
  },

  async cancelRecording(input: { testCaseId: string }): Promise<{ cancelled: true }> {
    const state = readStoredState();
    writeStoredState({
      ...state,
      recordingStatus: "idle"
    });

    dispatchEvent("browser.recording.status.changed", {
      testCaseId: input.testCaseId,
      status: "idle"
    });

    return { cancelled: true };
  }
};
