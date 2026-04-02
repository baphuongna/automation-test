import { useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import {
  getWebRecorderPreviewBanner,
  getWebRecorderWorkspaceBanner,
  webRecorderClient
} from "../services/web-recorder-client";
import { useTauriEvent } from "../hooks/useTauriEvent";
import { useRunStore } from "../store/run-store";
import type {
  BrowserHealthDto,
  RecordingStatus,
  StepAction,
  StepConfidence,
  UiStepDto,
  UiTestCaseDto
} from "../types";

interface RecorderFormState {
  id: string;
  name: string;
  startUrl: string;
}

interface RecorderStepDraft {
  id: string;
  action: StepAction;
  selector: string;
  value: string;
  timeoutMs: string;
  confidence: StepConfidence;
}

const STEP_ACTION_OPTIONS: Array<{ value: StepAction; label: string; helper: string }> = [
  { value: "navigate", label: "Navigate", helper: "Open a full URL in the browser session." },
  { value: "click", label: "Click", helper: "Click an element with a stable selector." },
  { value: "fill", label: "Fill", helper: "Type a value into an input or textarea." },
  { value: "select", label: "Select", helper: "Choose an option in a select element." },
  { value: "check", label: "Check", helper: "Enable a checkbox or toggle." },
  { value: "uncheck", label: "Uncheck", helper: "Disable a checkbox or toggle." },
  { value: "wait_for", label: "Wait for", helper: "Wait for an element or text to appear." },
  { value: "assert_text", label: "Assert text", helper: "Verify a piece of visible text." }
];

const STEP_ACTIONS_WITH_SELECTOR = new Set<StepAction>([
  "click",
  "fill",
  "select",
  "check",
  "uncheck",
  "wait_for",
  "assert_text"
]);

const STEP_ACTIONS_WITH_VALUE = new Set<StepAction>(["navigate", "fill", "select", "wait_for", "assert_text"]);

const EMPTY_DRAFT: UiTestCaseDto = {
  id: "ui-recorder-draft",
  type: "ui",
  name: "Untitled recorder draft",
  startUrl: "",
  steps: []
};

function createStepId(): string {
  return `ui-step-${Math.random().toString(36).slice(2, 10)}`;
}

function createEmptyStep(action: StepAction = "click"): RecorderStepDraft {
  return {
    id: createStepId(),
    action,
    selector: "",
    value: "",
    timeoutMs: "5000",
    confidence: "medium"
  };
}

function createFormState(draft: UiTestCaseDto): RecorderFormState {
  return {
    id: draft.id,
    name: draft.name,
    startUrl: draft.startUrl
  };
}

function inferConfidence(step: Pick<UiStepDto, "action" | "selector" | "value">): StepConfidence {
  const selector = step.selector?.trim();
  const value = step.value?.trim();
  const strongSelector = !!selector && (selector.startsWith("#") || selector.includes("data-testid") || selector.includes("[name="));
  const weakSelector = !!selector && (selector.startsWith(".") || selector.includes("nth-child") || selector.includes(":nth"));

  switch (step.action) {
    case "navigate":
      return value?.startsWith("http://") || value?.startsWith("https://") ? "high" : "medium";
    case "click":
    case "select":
    case "check":
    case "uncheck":
      if (strongSelector) {
        return "high";
      }
      return selector && !weakSelector ? "medium" : "low";
    case "fill":
    case "assert_text":
      if (strongSelector && value) {
        return "high";
      }
      return selector || value ? "medium" : "low";
    case "wait_for":
      return strongSelector || !!value ? "medium" : "low";
    default:
      return "medium";
  }
}

function createStepDraft(step: UiStepDto): RecorderStepDraft {
  return {
    id: step.id,
    action: step.action,
    selector: step.selector ?? "",
    value: step.value ?? "",
    timeoutMs: String(step.timeoutMs ?? 5000),
    confidence: step.confidence ?? inferConfidence(step)
  };
}

function toPositiveTimeout(value: string): number | undefined {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return undefined;
  }

  return parsed;
}

function buildStepPayload(step: RecorderStepDraft): UiStepDto {
  const selector = step.selector.trim();
  const value = step.value.trim();
  const timeoutMs = toPositiveTimeout(step.timeoutMs);
  const confidence = inferConfidence({
    action: step.action,
    ...(selector ? { selector } : {}),
    ...(value ? { value } : {})
  });

  return {
    id: step.id,
    action: step.action,
    ...(selector ? { selector } : {}),
    ...(value ? { value } : {}),
    ...(timeoutMs ? { timeoutMs } : {}),
    confidence
  };
}

function buildDraftPayload(form: RecorderFormState, steps: RecorderStepDraft[]): UiTestCaseDto {
  return {
    id: form.id.trim().length > 0 ? form.id.trim() : EMPTY_DRAFT.id,
    type: "ui",
    name: form.name.trim().length > 0 ? form.name.trim() : EMPTY_DRAFT.name,
    startUrl: form.startUrl.trim(),
    steps: steps.map(buildStepPayload)
  };
}

function moveItem<T>(items: T[], fromIndex: number, toIndex: number): T[] {
  const next = [...items];
  const [item] = next.splice(fromIndex, 1);
  if (item === undefined) {
    return items;
  }

  next.splice(toIndex, 0, item);
  return next;
}

function getStatusTone(status: RecordingStatus): {
  label: string;
  badgeModifier: string;
  panelModifier: string;
  description: string;
} {
  if (status === "recording") {
    return {
      label: "Recording",
      badgeModifier: " web-recorder__status-badge--recording",
      panelModifier: " web-recorder__status-panel--recording",
      description: "Live capture is active. Incoming steps will appear in the realtime stream and editor immediately."
    };
  }

  if (status === "failed") {
    return {
      label: "Failed",
      badgeModifier: " web-recorder__status-badge--failed",
      panelModifier: " web-recorder__status-panel--failed",
      description: "The recorder session stopped unexpectedly. Review the recoverable failure details and retry when ready."
    };
  }

  if (status === "stopped") {
    return {
      label: "Stopped",
      badgeModifier: " web-recorder__status-badge--stopped",
      panelModifier: "",
      description: "The last capture finished and the normalized steps are ready for editing and save."
    };
  }

  return {
    label: "Idle",
    badgeModifier: "",
    panelModifier: "",
    description: "Recorder is ready. Run preflight, confirm the draft URL, then start a new session."
  };
}

function formatActionLabel(action: StepAction): string {
  return STEP_ACTION_OPTIONS.find((option) => option.value === action)?.label ?? action;
}

function getConfidenceLabel(confidence: StepConfidence): string {
  if (confidence === "high") {
    return "High confidence";
  }

  if (confidence === "medium") {
    return "Medium confidence";
  }

  return "Low confidence";
}

function getStepSummary(step: RecorderStepDraft): string {
  if (step.action === "navigate") {
    return step.value.trim().length > 0 ? step.value.trim() : "Target URL missing";
  }

  if (step.selector.trim().length > 0) {
    return step.selector.trim();
  }

  if (step.value.trim().length > 0) {
    return step.value.trim();
  }

  return "Configuration incomplete";
}

export default function WebRecorder() {
  const { status: runStatus } = useRunStore();
  const [draft, setDraft] = useState<UiTestCaseDto>(EMPTY_DRAFT);
  const [form, setForm] = useState<RecorderFormState>(createFormState(EMPTY_DRAFT));
  const [stepDrafts, setStepDrafts] = useState<RecorderStepDraft[]>([]);
  const [selectedStepId, setSelectedStepId] = useState<string | null>(null);
  const [browserHealth, setBrowserHealth] = useState<BrowserHealthDto | null>(null);
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus>("idle");
  const [isLoading, setIsLoading] = useState(true);
  const [isCheckingHealth, setIsCheckingHealth] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [recoverableMessage, setRecoverableMessage] = useState<string | null>(null);

  const previewBanner = getWebRecorderPreviewBanner();
  const workspaceBanner = getWebRecorderWorkspaceBanner();
  const hasDraft = form.startUrl.trim().length > 0 || form.name.trim().length > 0 || stepDrafts.length > 0;
  const selectedStep = useMemo(
    () => stepDrafts.find((step) => step.id === selectedStepId) ?? stepDrafts[0] ?? null,
    [selectedStepId, stepDrafts]
  );
  const lowConfidenceCount = useMemo(
    () => stepDrafts.filter((step) => inferConfidence(step) === "low").length,
    [stepDrafts]
  );
  const statusTone = getStatusTone(recordingStatus);
  const isConflictBlocked = runStatus !== "idle";
  const isBrowserUnavailable = browserHealth?.runtimeStatus === "unavailable";
  const isRecordActionBlocked = isConflictBlocked || isBrowserUnavailable || recordingStatus === "recording";
  const isAnyMutationRunning = isSaving || isDeleting || isStarting || isStopping || isCancelling;

  useEffect(() => {
    void loadWorkspace();
  }, []);

  useEffect(() => {
    if (!selectedStep && stepDrafts.length > 0) {
      setSelectedStepId(stepDrafts[0]?.id ?? null);
      return;
    }

    if (selectedStepId && !stepDrafts.some((step) => step.id === selectedStepId)) {
      setSelectedStepId(stepDrafts[0]?.id ?? null);
    }
  }, [selectedStep, selectedStepId, stepDrafts]);

  useTauriEvent("browser.health.changed", (payload) => {
    setBrowserHealth(payload);
  });

  useTauriEvent("browser.recording.status.changed", (payload) => {
    if (payload.testCaseId !== form.id) {
      return;
    }

    setRecordingStatus(payload.status);
    if (payload.status === "failed") {
      setRecoverableMessage("Recorder session reported a recoverable failure. Review selectors and preflight status before retrying.");
      setErrorMessage("Recorder session failed.");
    }
  });

  useTauriEvent("browser.recording.step.captured", (payload) => {
    if (payload.testCaseId !== form.id) {
      return;
    }

    const capturedStep = createStepDraft(payload.step);
    setStepDrafts((current) => [...current, capturedStep]);
    setSelectedStepId(capturedStep.id);
    setFeedbackMessage(`Captured step ${formatActionLabel(capturedStep.action)}.`);
  });

  async function loadWorkspace(): Promise<void> {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const [workspaceDraft, health] = await Promise.all([webRecorderClient.loadWorkspace(), webRecorderClient.checkHealth()]);
      setDraft(workspaceDraft);
      setForm(createFormState(workspaceDraft));
      setStepDrafts(workspaceDraft.steps.map(createStepDraft));
      setSelectedStepId(workspaceDraft.steps[0]?.id ?? null);
      setBrowserHealth(health);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể tải Web Recorder workspace.";
      setErrorMessage(message);
    } finally {
      setIsLoading(false);
    }
  }

  function handleFormChange<Key extends keyof RecorderFormState>(key: Key, value: RecorderFormState[Key]): void {
    setForm((current) => ({
      ...current,
      [key]: value
    }));
  }

  function handleStepChange(stepId: string, patch: Partial<RecorderStepDraft>): void {
    setStepDrafts((current) =>
      current.map((step) => {
        if (step.id !== stepId) {
          return step;
        }

        const next = { ...step, ...patch };
        next.confidence = inferConfidence(next);
        return next;
      })
    );
  }

  function handleAddStep(): void {
    const nextStep = createEmptyStep();
    setStepDrafts((current) => [...current, nextStep]);
    setSelectedStepId(nextStep.id);
    setFeedbackMessage("Add step ready in the editor.");
  }

  function handleDeleteStep(stepId: string): void {
    setStepDrafts((current) => current.filter((step) => step.id !== stepId));
    setFeedbackMessage("Step deleted from the draft.");
  }

  function handleMoveStep(stepId: string, direction: -1 | 1): void {
    setStepDrafts((current) => {
      const index = current.findIndex((step) => step.id === stepId);
      if (index < 0) {
        return current;
      }

      const nextIndex = index + direction;
      if (nextIndex < 0 || nextIndex >= current.length) {
        return current;
      }

      return moveItem(current, index, nextIndex);
    });
  }

  async function handleCheckHealth(): Promise<void> {
    setIsCheckingHealth(true);
    setErrorMessage(null);

    try {
      const health = await webRecorderClient.checkHealth();
      setBrowserHealth(health);
      setFeedbackMessage("Preflight completed. Browser readiness has been refreshed.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể kiểm tra browser preflight.";
      setErrorMessage(message);
    } finally {
      setIsCheckingHealth(false);
    }
  }

  async function handleSave(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    const payload = buildDraftPayload(form, stepDrafts);
    if (payload.startUrl.trim().length === 0) {
      setErrorMessage("Start URL is required before saving the draft.");
      return;
    }

    setIsSaving(true);
    setErrorMessage(null);

    try {
      const saved = await webRecorderClient.upsert(payload);
      setDraft(saved);
      setForm(createFormState(saved));
      setStepDrafts(saved.steps.map(createStepDraft));
      setSelectedStepId(saved.steps[0]?.id ?? null);
      setFeedbackMessage("Recorder draft saved.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu recorder draft.";
      setErrorMessage(message);
    } finally {
      setIsSaving(false);
    }
  }

  async function handleDeleteDraft(): Promise<void> {
    if (!window.confirm(`Delete web recorder draft '${form.name.trim() || "Untitled recorder draft"}'?`)) {
      return;
    }

    setIsDeleting(true);
    setErrorMessage(null);

    try {
      await webRecorderClient.delete(form.id);
      setDraft(EMPTY_DRAFT);
      setForm(createFormState(EMPTY_DRAFT));
      setStepDrafts([]);
      setSelectedStepId(null);
      setRecordingStatus("idle");
      setFeedbackMessage("Recorder draft deleted.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa recorder draft.";
      setErrorMessage(message);
    } finally {
      setIsDeleting(false);
    }
  }

  async function handleStartRecording(): Promise<void> {
    if (isConflictBlocked) {
      setErrorMessage("Conflict blocked: replay/run is active, so recording cannot start right now.");
      return;
    }

    const payload = buildDraftPayload(form, stepDrafts);
    if (payload.startUrl.trim().length === 0) {
      setErrorMessage("Start URL is required before starting recording.");
      return;
    }

    setIsStarting(true);
    setErrorMessage(null);
    setRecoverableMessage(null);

    try {
      await webRecorderClient.upsert(payload);
      await webRecorderClient.startRecording({
        testCaseId: payload.id,
        startUrl: payload.startUrl
      });
      setRecordingStatus("recording");
      setFeedbackMessage("Recording started. Live steps will stream into the editor.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể bắt đầu recorder session.";
      setErrorMessage(message);
    } finally {
      setIsStarting(false);
    }
  }

  async function handleStopRecording(): Promise<void> {
    setIsStopping(true);
    setErrorMessage(null);

    try {
      const saved = await webRecorderClient.stopRecording({ testCaseId: form.id });
      setDraft(saved);
      setForm(createFormState(saved));
      setStepDrafts(saved.steps.map(createStepDraft));
      setSelectedStepId(saved.steps[0]?.id ?? null);
      setRecordingStatus("stopped");
      setFeedbackMessage("Recording stopped and normalized draft saved.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể dừng recorder session.";
      setErrorMessage(message);
    } finally {
      setIsStopping(false);
    }
  }

  async function handleCancelRecording(): Promise<void> {
    setIsCancelling(true);
    setErrorMessage(null);

    try {
      await webRecorderClient.cancelRecording({ testCaseId: form.id });
      setRecordingStatus("idle");
      setFeedbackMessage("Recorder session cancelled.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể hủy recorder session.";
      setErrorMessage(message);
    } finally {
      setIsCancelling(false);
    }
  }

  if (isLoading) {
    return (
      <section className="web-recorder" data-testid="route-web-recorder">
        <div className="web-recorder__hero">
          <div>
            <span className="web-recorder__eyebrow">Loading</span>
            <h1>Web Recorder</h1>
            <p>Loading recorder workspace, browser preflight snapshot, and the last draft.</p>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="web-recorder" data-testid="route-web-recorder">
      <header className="web-recorder__hero">
        <div>
          <span className="web-recorder__eyebrow">Web Recorder</span>
          <h1>Recorder workspace for live browser capture and step editing</h1>
          <p>
            Run browser preflight, manage the active recorder session, and refine Phase 1 basic steps with
            selector, value, timeout, add, delete, and reorder controls.
          </p>
        </div>

        <div className="web-recorder__hero-actions">
          <button
            type="button"
            className="web-recorder__secondary-action"
            onClick={() => void handleCheckHealth()}
            disabled={isCheckingHealth || isAnyMutationRunning}
          >
            {isCheckingHealth ? "Checking preflight..." : "Run preflight"}
          </button>
          <button
            type="button"
            className="web-recorder__primary-action"
            onClick={() => void handleStartRecording()}
            disabled={isRecordActionBlocked || isAnyMutationRunning}
          >
            {isStarting ? "Starting..." : "Start recording"}
          </button>
          <button
            type="button"
            className="web-recorder__secondary-action"
            onClick={() => void handleStopRecording()}
            disabled={recordingStatus !== "recording" || isAnyMutationRunning}
          >
            {isStopping ? "Stopping..." : "Stop and save"}
          </button>
          <button
            type="button"
            className="web-recorder__danger-action"
            onClick={() => void handleCancelRecording()}
            disabled={recordingStatus !== "recording" || isAnyMutationRunning}
          >
            {isCancelling ? "Cancelling..." : "Cancel session"}
          </button>
        </div>
      </header>

      {previewBanner ? <div className="web-recorder__feedback">{previewBanner}</div> : null}
      {workspaceBanner ? <div className="web-recorder__feedback web-recorder__feedback--warning">{workspaceBanner}</div> : null}
      {feedbackMessage ? <div className="web-recorder__feedback">{feedbackMessage}</div> : null}
      {recoverableMessage ? (
        <div className="web-recorder__feedback web-recorder__feedback--warning">
          <strong>recoverable</strong>
          <span>{recoverableMessage}</span>
        </div>
      ) : null}
      {errorMessage ? <div className="web-recorder__feedback web-recorder__feedback--error">{errorMessage}</div> : null}

      {isConflictBlocked ? (
        <div className="web-recorder__feedback web-recorder__feedback--warning">
          <strong>Conflict blocked</strong>
          <span>Replay/run is currently {runStatus}, so recording actions stay visually blocked until runtime returns to idle.</span>
        </div>
      ) : null}

      <div className="web-recorder__layout">
        <aside className="web-recorder__panel web-recorder__panel--sidebar">
          <div className={`web-recorder__status-panel${statusTone.panelModifier}`}>
            <div className="web-recorder__panel-header">
              <h2>Preflight</h2>
              <span className={`web-recorder__status-badge${isConflictBlocked ? " web-recorder__status-badge--blocked" : statusTone.badgeModifier}`}>
                {isConflictBlocked ? "Conflict blocked" : statusTone.label}
              </span>
            </div>

            <dl className="web-recorder__metric-grid">
              <div>
                <dt>Runtime</dt>
                <dd>{browserHealth?.runtimeStatus ?? "unknown"}</dd>
              </div>
              <div>
                <dt>Checked</dt>
                <dd>{browserHealth?.checkedAt ?? "Not checked"}</dd>
              </div>
              <div>
                <dt>Low confidence</dt>
                <dd>{lowConfidenceCount}</dd>
              </div>
            </dl>

            <p>{browserHealth?.message ?? "Run preflight to capture browser readiness."}</p>
          </div>

          <div className={`web-recorder__status-panel${statusTone.panelModifier}`}>
            <div className="web-recorder__panel-header">
              <h2>Session status</h2>
              <span className={`web-recorder__status-badge${statusTone.badgeModifier}`}>{statusTone.label}</span>
            </div>
            <p>{statusTone.description}</p>
            <ul className="web-recorder__status-list">
              <li>Draft id: {form.id}</li>
              <li>Steps in editor: {stepDrafts.length}</li>
              <li>Replay/run status: {runStatus}</li>
            </ul>
          </div>

          <div className="web-recorder__panel">
            <div className="web-recorder__panel-header">
              <h2>Captured steps</h2>
              <span>Realtime step stream</span>
            </div>

            {stepDrafts.length === 0 ? (
              <div className="web-recorder__empty-panel">No draft yet. Start recording or add a manual step to begin editing.</div>
            ) : (
              <div className="web-recorder__step-list" aria-label="Captured step stream">
                {stepDrafts.map((step, index) => {
                  const confidence = inferConfidence(step);

                  return (
                    <button
                      key={step.id}
                      type="button"
                      className={`web-recorder__step-card${selectedStep?.id === step.id ? " web-recorder__step-card--active" : ""}${confidence === "low" ? " web-recorder__step--low-confidence" : ""}`}
                      onClick={() => setSelectedStepId(step.id)}
                    >
                      <div className="web-recorder__step-card-header">
                        <strong>
                          {index + 1}. {formatActionLabel(step.action)}
                        </strong>
                        <span className={`web-recorder__confidence-badge web-recorder__confidence-badge--${confidence}`}>
                          {getConfidenceLabel(confidence)}
                        </span>
                      </div>
                      <p>{getStepSummary(step)}</p>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </aside>

        <form className="web-recorder__panel web-recorder__editor" onSubmit={(event) => void handleSave(event)}>
          <div className="web-recorder__panel-header">
            <h2>Step editor</h2>
            <span>Phase 1 basic actions only</span>
          </div>

          <div className="web-recorder__field-row">
            <label className="web-recorder__field web-recorder__field--wide">
              <span>Scenario name</span>
              <input value={form.name} onChange={(event) => handleFormChange("name", event.target.value)} />
            </label>

            <label className="web-recorder__field web-recorder__field--wide">
              <span>Start URL</span>
              <input
                value={form.startUrl}
                onChange={(event) => handleFormChange("startUrl", event.target.value)}
                placeholder="https://app.testforge.local/login"
              />
            </label>
          </div>

          <div className="web-recorder__editor-actions">
            <button type="button" className="web-recorder__secondary-action" onClick={handleAddStep} disabled={isAnyMutationRunning}>
              Add step
            </button>
            <button type="submit" className="web-recorder__primary-action" disabled={isAnyMutationRunning}>
              {isSaving ? "Saving..." : "Save draft"}
            </button>
            <button
              type="button"
              className="web-recorder__danger-action"
              onClick={() => void handleDeleteDraft()}
              disabled={isDeleting || isAnyMutationRunning}
            >
              {isDeleting ? "Deleting..." : "Delete draft"}
            </button>
          </div>

          {!hasDraft ? <div className="web-recorder__empty-panel">No draft yet. Save a start URL or capture steps to create a recorder draft.</div> : null}

          <div className="web-recorder__editor-grid">
            <section className="web-recorder__subpanel">
              <div className="web-recorder__panel-header">
                <h3>Editor queue</h3>
                <span>{stepDrafts.length} steps</span>
              </div>

              {stepDrafts.length === 0 ? (
                <div className="web-recorder__empty-panel">Add step to start a manual sequence.</div>
              ) : (
                <div className="web-recorder__step-order-list">
                  {stepDrafts.map((step, index) => {
                    const confidence = inferConfidence(step);

                    return (
                      <div
                        key={step.id}
                        className={`web-recorder__step-order-card${selectedStep?.id === step.id ? " web-recorder__step-order-card--active" : ""}${confidence === "low" ? " web-recorder__step--low-confidence" : ""}`}
                      >
                        <button type="button" className="web-recorder__step-link" onClick={() => setSelectedStepId(step.id)}>
                          <strong>
                            {index + 1}. {formatActionLabel(step.action)}
                          </strong>
                          <span>{getStepSummary(step)}</span>
                        </button>

                        <div className="web-recorder__step-toolbar">
                          <button type="button" className="web-recorder__ghost-action" onClick={() => handleMoveStep(step.id, -1)} disabled={index === 0}>
                            Move up
                          </button>
                          <button
                            type="button"
                            className="web-recorder__ghost-action"
                            onClick={() => handleMoveStep(step.id, 1)}
                            disabled={index === stepDrafts.length - 1}
                          >
                            Move down
                          </button>
                          <button type="button" className="web-recorder__ghost-action" onClick={() => handleDeleteStep(step.id)}>
                            Delete
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>

            <section className="web-recorder__subpanel">
              <div className="web-recorder__panel-header">
                <h3>Selected step</h3>
                <span>{selectedStep ? getConfidenceLabel(inferConfidence(selectedStep)) : "No step selected"}</span>
              </div>

              {selectedStep ? (
                <div className="web-recorder__step-detail">
                  <label className="web-recorder__field">
                    <span>Action</span>
                    <select
                      value={selectedStep.action}
                      onChange={(event) => handleStepChange(selectedStep.id, { action: event.target.value as StepAction })}
                    >
                      {STEP_ACTION_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>

                  <p className="web-recorder__hint">
                    {STEP_ACTION_OPTIONS.find((option) => option.value === selectedStep.action)?.helper}
                  </p>

                  <div className="web-recorder__field-row">
                    <label className="web-recorder__field web-recorder__field--wide">
                      <span>selector</span>
                      <input
                        value={selectedStep.selector}
                        onChange={(event) => handleStepChange(selectedStep.id, { selector: event.target.value })}
                        placeholder={STEP_ACTIONS_WITH_SELECTOR.has(selectedStep.action) ? "button[data-testid=submit]" : "Optional for this action"}
                        disabled={!STEP_ACTIONS_WITH_SELECTOR.has(selectedStep.action)}
                      />
                    </label>

                    <label className="web-recorder__field web-recorder__field--wide">
                      <span>value</span>
                      <input
                        value={selectedStep.value}
                        onChange={(event) => handleStepChange(selectedStep.id, { value: event.target.value })}
                        placeholder={STEP_ACTIONS_WITH_VALUE.has(selectedStep.action) ? "Value or visible text" : "Optional for this action"}
                        disabled={!STEP_ACTIONS_WITH_VALUE.has(selectedStep.action)}
                      />
                    </label>
                  </div>

                  <div className="web-recorder__field-row">
                    <label className="web-recorder__field web-recorder__field--compact">
                      <span>timeoutMs</span>
                      <input
                        value={selectedStep.timeoutMs}
                        onChange={(event) => handleStepChange(selectedStep.id, { timeoutMs: event.target.value })}
                      />
                    </label>

                    <div className="web-recorder__confidence-panel">
                      <span className={`web-recorder__confidence-badge web-recorder__confidence-badge--${inferConfidence(selectedStep)}`}>
                        {getConfidenceLabel(inferConfidence(selectedStep))}
                      </span>
                      <p>
                        {inferConfidence(selectedStep) === "low"
                          ? "Low confidence selector/value detected. Stabilize this step before replay runs in desktop runtime."
                          : "Confidence is derived from the current selector/value quality and updates in realtime."}
                      </p>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="web-recorder__empty-panel">Select a captured step to edit selector, value, timeout, and order.</div>
              )}
            </section>
          </div>
        </form>
      </div>

      <div className="web-recorder__panel">
        <div className="web-recorder__panel-header">
          <h2>Draft snapshot</h2>
          <span>Last persisted workspace state</span>
        </div>

        <dl className="web-recorder__request-grid">
          <div>
            <dt>Name</dt>
            <dd>{draft.name}</dd>
          </div>
          <div>
            <dt>Start URL</dt>
            <dd>{draft.startUrl || "No draft yet"}</dd>
          </div>
          <div>
            <dt>Steps</dt>
            <dd>{draft.steps.length}</dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
