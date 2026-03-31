import type { EnvironmentDto, EnvironmentVariableDto } from "../types";

const PREVIEW_STORAGE_KEY = "testforge.environmentManager.preview.v1";
const PREVIEW_DEGRADED_QUERY = "previewDegraded";
const PREVIEW_DEGRADED_STORAGE_KEY = "testforge.environmentManager.preview.degraded";
const MASKED_BY_DEFAULT_NOTE = "masked by default";

interface PreviewEnvironmentState {
  environments: EnvironmentDto[];
}

interface EnvironmentSaveInput {
  id?: string;
  name: string;
  envType: EnvironmentDto["envType"];
  isDefault: boolean;
}

interface EnvironmentVariableSaveInput {
  environmentId: string;
  id?: string;
  key: string;
  kind: EnvironmentVariableDto["kind"];
  value: string;
}

function nowIso(): string {
  return new Date().toISOString();
}

function createId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function getMaskedPreview(value: string): string {
  const trimmedValue = value.trim();

  if (trimmedValue.length <= 4) {
    return "••••";
  }

  return `${trimmedValue.slice(0, 2)}••••${trimmedValue.slice(-2)}`;
}

function createSeedState(): PreviewEnvironmentState {
  const createdAt = nowIso();

  return {
    environments: [
      {
        id: "env-preview-dev",
        name: "Development Preview",
        envType: "development",
        isDefault: true,
        createdAt,
        updatedAt: createdAt,
        variables: [
          {
            id: "var-preview-base-url",
            key: "BASE_URL",
            kind: "plain",
            valueMaskedPreview: "https://preview.dev.local"
          }
        ]
      },
      {
        id: "env-preview-prod",
        name: "Production Preview",
        envType: "production",
        isDefault: false,
        createdAt,
        updatedAt: createdAt,
        variables: [
          {
            id: "var-preview-api-key",
            key: "API_KEY",
            kind: "secret",
            valueMaskedPreview: `pr••••ew (${MASKED_BY_DEFAULT_NOTE})`
          }
        ]
      }
    ]
  };
}

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readStoredState(): PreviewEnvironmentState {
  if (!canUseBrowserStorage()) {
    return createSeedState();
  }

  const raw = window.localStorage.getItem(PREVIEW_STORAGE_KEY);
  if (!raw) {
    const seedState = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seedState));
    return seedState;
  }

  try {
    return JSON.parse(raw) as PreviewEnvironmentState;
  } catch {
    const seedState = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seedState));
    return seedState;
  }
}

function writeStoredState(state: PreviewEnvironmentState): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(state));
}

function isPreviewDegradedEnabled(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const query = new URLSearchParams(window.location.search);
  const previewDegraded = query.get(PREVIEW_DEGRADED_QUERY);
  if (previewDegraded === "1" || previewDegraded === "true") {
    return true;
  }

  if (!canUseBrowserStorage()) {
    return false;
  }

  const stored = window.localStorage.getItem(PREVIEW_DEGRADED_STORAGE_KEY);
  return stored === "1" || stored === "true";
}

function ensureSingleDefault(environments: EnvironmentDto[], defaultId: string | null): EnvironmentDto[] {
  return environments.map((environment) => ({
    ...environment,
    isDefault: environment.id === defaultId
  }));
}

function getDegradedPreviewVariable(variable: EnvironmentVariableDto): EnvironmentVariableDto {
  if (variable.kind !== "secret") {
    return variable;
  }

  return {
    ...variable,
    valueMaskedPreview: `${variable.valueMaskedPreview}`
  };
}

export const environmentPreviewClient = {
  banner: "Preview fallback active - browser-only T6 verification path.",

  isAvailable(): boolean {
    return typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
  },

  isDegraded(): boolean {
    return isPreviewDegradedEnabled();
  },

  list(): Promise<EnvironmentDto[]> {
    const state = readStoredState();
    return Promise.resolve(
      state.environments.map((environment) => ({
        ...environment,
        variables: environment.variables.map(getDegradedPreviewVariable)
      }))
    );
  },

  create(input: EnvironmentSaveInput): Promise<EnvironmentDto> {
    const state = readStoredState();
    const timestamp = nowIso();
    const environment: EnvironmentDto = {
      id: createId("env-preview"),
      name: input.name,
      envType: input.envType,
      isDefault: input.isDefault,
      createdAt: timestamp,
      updatedAt: timestamp,
      variables: []
    };

    const environments = input.isDefault
      ? ensureSingleDefault([...state.environments, environment], environment.id)
      : [...state.environments, environment];

    writeStoredState({ environments });
    return Promise.resolve(environment);
  },

  update(input: EnvironmentSaveInput & { id: string }): Promise<EnvironmentDto> {
    const state = readStoredState();
    const timestamp = nowIso();

    const environments = state.environments.map((environment) =>
      environment.id === input.id
        ? {
            ...environment,
            name: input.name,
            envType: input.envType,
            isDefault: input.isDefault,
            updatedAt: timestamp
          }
        : environment
    );

    const normalized = input.isDefault ? ensureSingleDefault(environments, input.id) : environments;
    writeStoredState({ environments: normalized });

    const updated = normalized.find((environment) => environment.id === input.id);
    if (!updated) {
      throw new Error("Không tìm thấy môi trường preview để cập nhật.");
    }

    return Promise.resolve(updated);
  },

  remove(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    const environments = state.environments.filter((environment) => environment.id !== id);
    const defaultId = environments.find((environment) => environment.isDefault)?.id ?? environments[0]?.id ?? null;
    writeStoredState({ environments: ensureSingleDefault(environments, defaultId) });
    return Promise.resolve({ deleted: true });
  },

  upsertVariable(input: EnvironmentVariableSaveInput): Promise<EnvironmentVariableDto> {
    if (input.kind === "secret" && isPreviewDegradedEnabled()) {
      return Promise.reject({
        code: "SECRET_KEY_MISSING",
        context: { command: "environment.variable.upsert", preview: true },
        displayMessage: "Secret store degraded trong browser preview.",
        recoverable: true,
        technicalMessage: "Preview fallback blocked secret upsert because previewDegraded is enabled."
      });
    }

    const state = readStoredState();
    const timestamp = nowIso();
    let savedVariable: EnvironmentVariableDto | null = null;

    const environments = state.environments.map((environment) => {
      if (environment.id !== input.environmentId) {
        return environment;
      }

      const variable: EnvironmentVariableDto = {
        id: input.id ?? createId("var-preview"),
        key: input.key,
        kind: input.kind,
        valueMaskedPreview:
          input.kind === "secret"
            ? `${getMaskedPreview(input.value)} (${MASKED_BY_DEFAULT_NOTE})`
            : input.value
      };

      const variables = input.id
        ? environment.variables.map((current) => (current.id === input.id ? variable : current))
        : [...environment.variables, variable];

      savedVariable = variable;

      return {
        ...environment,
        updatedAt: timestamp,
        variables
      };
    });

    writeStoredState({ environments });

    if (!savedVariable) {
      throw new Error("Không tìm thấy môi trường preview để lưu biến.");
    }

    return Promise.resolve(savedVariable);
  },

  deleteVariable(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    const environments = state.environments.map((environment) => ({
      ...environment,
      variables: environment.variables.filter((variable) => variable.id !== id)
    }));

    writeStoredState({ environments });
    return Promise.resolve({ deleted: true });
  }
};
