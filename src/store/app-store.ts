import { create } from "zustand";
import type { BrowserRuntimeStatus } from "../types";

interface AppState {
  isShellReady: boolean;
  lastErrorMessage: string | null;
  browserRuntimeStatus: BrowserRuntimeStatus;
  setShellReady: (isReady: boolean) => void;
  setLastErrorMessage: (message: string | null) => void;
  setBrowserRuntimeStatus: (status: BrowserRuntimeStatus) => void;
  reset: () => void;
}

const initialAppState = {
  browserRuntimeStatus: "healthy" as BrowserRuntimeStatus,
  isShellReady: false,
  lastErrorMessage: null as string | null
};

export const useAppStore = create<AppState>((set) => ({
  ...initialAppState,
  setShellReady: (isReady) => {
    set({ isShellReady: isReady });
  },
  setLastErrorMessage: (message) => {
    set({ lastErrorMessage: message });
  },
  setBrowserRuntimeStatus: (status) => {
    set({ browserRuntimeStatus: status });
  },
  reset: () => {
    set(initialAppState);
  }
}));
