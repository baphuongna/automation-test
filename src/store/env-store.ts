import { create } from "zustand";

export interface EnvironmentSummary {
  id: string;
  name: string;
  envType: "development" | "staging" | "production" | "custom";
  isDefault: boolean;
}

interface EnvironmentState {
  activeEnvironmentId: string | null;
  environments: EnvironmentSummary[];
  setEnvironments: (environments: EnvironmentSummary[]) => void;
  setActiveEnvironmentId: (environmentId: string | null) => void;
  reset: () => void;
}

const initialEnvironmentState = {
  activeEnvironmentId: null,
  environments: [] as EnvironmentSummary[]
};

export const useEnvStore = create<EnvironmentState>((set) => ({
  ...initialEnvironmentState,
  setEnvironments: (environments) => {
    set((state) => {
      const activeEnvironmentExists = environments.some(
        (environment) => environment.id === state.activeEnvironmentId
      );
      const defaultEnvironmentId = environments.find((environment) => environment.isDefault)?.id ?? null;

      return {
        activeEnvironmentId: activeEnvironmentExists ? state.activeEnvironmentId : defaultEnvironmentId,
        environments
      };
    });
  },
  setActiveEnvironmentId: (environmentId) => {
    set({ activeEnvironmentId: environmentId });
  },
  reset: () => {
    set(initialEnvironmentState);
  }
}));
