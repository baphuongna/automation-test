import { create } from "zustand";
import type { EventPayloadMap, RunStatus } from "../types";

export interface RunProgressSnapshot {
  completed: number;
  failed: number;
  skipped: number;
  passed: number;
  total: number;
}

interface RunState {
  activeRunId: string | null;
  isStopping: boolean;
  terminalMessage: string | null;
  progress: RunProgressSnapshot | null;
  status: RunStatus;
  setRunState: (payload: {
    activeRunId: string | null;
    isStopping: boolean;
    terminalMessage: string | null;
    progress: RunProgressSnapshot | null;
    status: RunStatus;
  }) => void;
  reset: () => void;
}

const initialRunState = {
  activeRunId: null,
  isStopping: false,
  terminalMessage: null,
  progress: null as RunProgressSnapshot | null,
  status: "idle" as RunStatus
};

export const useRunStore = create<RunState>((set) => ({
  ...initialRunState,
  setRunState: ({ activeRunId, isStopping, terminalMessage, progress, status }) => {
    set({ activeRunId, isStopping, terminalMessage, progress, status });
  },
  reset: () => {
    set(initialRunState);
  }
}));

export function subscribeRunnerEvents(handlers: {
  onStarted?: (payload: EventPayloadMap["runner.execution.started"]) => void;
  onProgress?: (payload: EventPayloadMap["runner.execution.progress"]) => void;
  onCompleted?: (payload: EventPayloadMap["runner.execution.completed"]) => void;
}): () => void {
  const startedListener = (event: Event): void => {
    const payload = (event as CustomEvent<EventPayloadMap["runner.execution.started"]>).detail;
    useRunStore.getState().setRunState({
      activeRunId: payload.runId,
      isStopping: false,
      terminalMessage: null,
      progress: null,
      status: "queued"
    });
    handlers.onStarted?.(payload);
  };

  const progressListener = (event: Event): void => {
    const payload = (event as CustomEvent<EventPayloadMap["runner.execution.progress"]>).detail;
    const currentState = useRunStore.getState();
    useRunStore.getState().setRunState({
      activeRunId: payload.runId,
      isStopping: currentState.isStopping,
      terminalMessage: null,
      progress: {
        completed: payload.completedCount,
        failed: payload.failedCount,
        skipped: payload.skippedCount,
        passed: payload.passedCount,
        total: payload.totalCount
      },
      status: "running"
    });
    handlers.onProgress?.(payload);
  };

  const completedListener = (event: Event): void => {
    const payload = (event as CustomEvent<EventPayloadMap["runner.execution.completed"]>).detail;
    useRunStore.getState().setRunState({
      activeRunId: null,
      isStopping: false,
      terminalMessage:
        payload.status === "cancelled"
          ? "Run cancelled safely. No active run remains."
          : `Run completed as ${payload.status}.`,
      progress: {
        completed: payload.passedCount + payload.failedCount + payload.skippedCount,
        failed: payload.failedCount,
        skipped: payload.skippedCount,
        passed: payload.passedCount,
        total: payload.totalCount
      },
      status: payload.status
    });
    handlers.onCompleted?.(payload);
  };

  if (typeof window === "undefined") {
    return () => undefined;
  }

  window.addEventListener("runner.execution.started", startedListener);
  window.addEventListener("runner.execution.progress", progressListener);
  window.addEventListener("runner.execution.completed", completedListener);

  return () => {
    window.removeEventListener("runner.execution.started", startedListener);
    window.removeEventListener("runner.execution.progress", progressListener);
    window.removeEventListener("runner.execution.completed", completedListener);
  };
}
