import { create } from "zustand";
import type { RunStatus } from "../types";

export interface RunProgressSnapshot {
  completed: number;
  failed: number;
  total: number;
}

interface RunState {
  activeRunId: string | null;
  isStopping: boolean;
  progress: RunProgressSnapshot | null;
  status: RunStatus;
  setRunState: (payload: {
    activeRunId: string | null;
    isStopping: boolean;
    progress: RunProgressSnapshot | null;
    status: RunStatus;
  }) => void;
  reset: () => void;
}

const initialRunState = {
  activeRunId: null,
  isStopping: false,
  progress: null as RunProgressSnapshot | null,
  status: "idle" as RunStatus
};

export const useRunStore = create<RunState>((set) => ({
  ...initialRunState,
  setRunState: ({ activeRunId, isStopping, progress, status }) => {
    set({ activeRunId, isStopping, progress, status });
  },
  reset: () => {
    set(initialRunState);
  }
}));
