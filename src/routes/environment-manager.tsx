import { useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import {
  environmentClient,
  getEnvironmentPreviewBanner,
  isEnvironmentPreviewDegradedMode,
  isSecretStoreBlockedError
} from "../services/environment-client";
import type { EnvironmentDto, EnvironmentVariableDto } from "../types";
import { useEnvStore } from "../store/env-store";

type EnvironmentTypeOption = EnvironmentDto["envType"];
type VariableKindOption = EnvironmentVariableDto["kind"];

interface EnvironmentFormState {
  id: string | null;
  name: string;
  envType: EnvironmentTypeOption;
  isDefault: boolean;
}

interface VariableFormState {
  id: string | null;
  key: string;
  kind: VariableKindOption;
  value: string;
}

const ENVIRONMENT_TYPE_OPTIONS: Array<{ label: string; value: EnvironmentTypeOption }> = [
  { label: "Development", value: "development" },
  { label: "Staging", value: "staging" },
  { label: "Production", value: "production" },
  { label: "Custom", value: "custom" }
];

const VARIABLE_KIND_OPTIONS: Array<{ label: string; value: VariableKindOption }> = [
  { label: "Plain text", value: "plain" },
  { label: "Secret", value: "secret" }
];

const EMPTY_ENVIRONMENT_FORM: EnvironmentFormState = {
  id: null,
  name: "",
  envType: "development",
  isDefault: false
};

const EMPTY_VARIABLE_FORM: VariableFormState = {
  id: null,
  key: "",
  kind: "plain",
  value: ""
};

function createEnvironmentSummary(environment: EnvironmentDto) {
  return {
    id: environment.id,
    name: environment.name,
    envType: environment.envType,
    isDefault: environment.isDefault
  };
}

function isProductionLikeEnvironment(environment: EnvironmentDto | null): boolean {
  return environment?.envType === "production";
}

function getEnvironmentTypeLabel(value: EnvironmentTypeOption): string {
  return ENVIRONMENT_TYPE_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

function getVariableKindLabel(value: VariableKindOption): string {
  return VARIABLE_KIND_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

function createDefaultEnvironmentName(value: EnvironmentTypeOption): string {
  return getEnvironmentTypeLabel(value);
}

export default function EnvironmentManager() {
  const { activeEnvironmentId, setActiveEnvironmentId, setEnvironments } = useEnvStore();
  const [environments, setEnvironmentRecords] = useState<EnvironmentDto[]>([]);
  const [environmentForm, setEnvironmentForm] = useState<EnvironmentFormState>(EMPTY_ENVIRONMENT_FORM);
  const [variableForm, setVariableForm] = useState<VariableFormState>(EMPTY_VARIABLE_FORM);
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingEnvironment, setIsSavingEnvironment] = useState(false);
  const [isSavingVariable, setIsSavingVariable] = useState(false);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [degradedMessage, setDegradedMessage] = useState<string | null>(null);
  const previewBanner = getEnvironmentPreviewBanner();

  const activeEnvironment = useMemo(
    () => environments.find((environment) => environment.id === activeEnvironmentId) ?? environments[0] ?? null,
    [activeEnvironmentId, environments]
  );

  useEffect(() => {
    void loadEnvironments();
  }, []);

  useEffect(() => {
    if (isEnvironmentPreviewDegradedMode()) {
      setDegradedMessage("Secret store degraded. Preview fallback is blocking secret writes deterministically.");
    }
  }, []);

  useEffect(() => {
    if (!activeEnvironment) {
      setEnvironmentForm(EMPTY_ENVIRONMENT_FORM);
      setVariableForm(EMPTY_VARIABLE_FORM);
      return;
    }

    setEnvironmentForm({
      id: activeEnvironment.id,
      name: activeEnvironment.name,
      envType: activeEnvironment.envType,
      isDefault: activeEnvironment.isDefault
    });
    setVariableForm(EMPTY_VARIABLE_FORM);
  }, [activeEnvironment]);

  async function loadEnvironments(): Promise<void> {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const records = await environmentClient.list();
      setEnvironmentRecords(records);
      setEnvironments(records.map(createEnvironmentSummary));

      const nextActiveId =
        records.find((environment) => environment.id === activeEnvironmentId)?.id ??
        records.find((environment) => environment.isDefault)?.id ??
        records[0]?.id ??
        null;

      setActiveEnvironmentId(nextActiveId);

      if (nextActiveId === null) {
        setEnvironmentForm(EMPTY_ENVIRONMENT_FORM);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể tải danh sách môi trường.";
      setErrorMessage(message);
    } finally {
      setIsLoading(false);
    }
  }

  async function handleEnvironmentSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    const trimmedName = environmentForm.name.trim() || createDefaultEnvironmentName(environmentForm.envType);
    const payload = {
      name: trimmedName,
      envType: environmentForm.envType,
      isDefault: environmentForm.isDefault
    };

    setIsSavingEnvironment(true);
    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      const saved = environmentForm.id
        ? await environmentClient.update({ ...payload, id: environmentForm.id })
        : await environmentClient.create(payload);

      await loadEnvironments();
      setActiveEnvironmentId(saved.id);
      setFeedbackMessage(environmentForm.id ? "Environment updated." : "Environment created.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu môi trường.";
      setErrorMessage(message);
    } finally {
      setIsSavingEnvironment(false);
    }
  }

  async function handleDeleteEnvironment(): Promise<void> {
    if (!activeEnvironment) {
      return;
    }

    const needsConfirmation = isProductionLikeEnvironment(activeEnvironment);
    const confirmationMessage = needsConfirmation
      ? `Production-like environment '${activeEnvironment.name}' will be deleted. Confirm destructive action?`
      : `Delete environment '${activeEnvironment.name}'?`;

    if (!window.confirm(confirmationMessage)) {
      return;
    }

    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      await environmentClient.remove(activeEnvironment.id);
      setFeedbackMessage("Environment deleted.");
      await loadEnvironments();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa môi trường.";
      setErrorMessage(message);
    }
  }

  function handleCreateEnvironmentDraft(): void {
    setActiveEnvironmentId(null);
    setEnvironmentForm(EMPTY_ENVIRONMENT_FORM);
    setVariableForm(EMPTY_VARIABLE_FORM);
    setFeedbackMessage(null);
    setErrorMessage(null);
  }

  async function handleVariableSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    if (!activeEnvironment) {
      setErrorMessage("Chọn một môi trường trước khi lưu biến.");
      return;
    }

    const trimmedKey = variableForm.key.trim();
    if (trimmedKey.length === 0) {
      setErrorMessage("Variable key is required.");
      return;
    }

    setIsSavingVariable(true);
    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      const payload = {
        environmentId: activeEnvironment.id,
        key: trimmedKey,
        kind: variableForm.kind,
        value: variableForm.value
      };

      await environmentClient.upsertVariable(
        variableForm.id
          ? {
              ...payload,
              id: variableForm.id
            }
          : payload
      );

      setVariableForm(EMPTY_VARIABLE_FORM);
      setFeedbackMessage(variableForm.id ? "Variable updated." : "Variable saved.");
      setDegradedMessage(null);
      await loadEnvironments();
      setActiveEnvironmentId(activeEnvironment.id);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu biến môi trường.";
      setErrorMessage(message);

      if (isSecretStoreBlockedError(error as never)) {
        setDegradedMessage("Secret store degraded. Secret operations are blocked until the master key is available again.");
      }
    } finally {
      setIsSavingVariable(false);
    }
  }

  async function handleDeleteVariable(variable: EnvironmentVariableDto): Promise<void> {
    const confirmationMessage = `Delete variable '${variable.key}'?`;
    if (!window.confirm(confirmationMessage)) {
      return;
    }

    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      await environmentClient.deleteVariable(variable.id);
      setFeedbackMessage("Variable deleted.");
      await loadEnvironments();
      if (activeEnvironment) {
        setActiveEnvironmentId(activeEnvironment.id);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa biến môi trường.";
      setErrorMessage(message);
    }
  }

  function handleEditVariable(variable: EnvironmentVariableDto): void {
    setVariableForm({
      id: variable.id,
      key: variable.key,
      kind: variable.kind,
      value: ""
    });
    setFeedbackMessage(
      variable.kind === "secret"
        ? "Secret values stay masked by default. Enter a new secret to rotate it."
        : `Editing variable '${variable.key}'.`
    );
  }

  return (
    <section className="environment-manager" data-testid="route-environment-manager">
      <header className="environment-manager__hero">
        <div>
          <span className="route-skeleton__eyebrow">Environment Manager</span>
          <h1>Manage environments, defaults, and secret-safe variables</h1>
          <p>
            Secrets remain masked by default on every list view. Use this screen to create, update,
            delete, and set the default environment without leaving the typed IPC boundary.
          </p>
        </div>
        <button type="button" className="environment-manager__primary-action" onClick={handleCreateEnvironmentDraft}>
          New environment
        </button>
      </header>

      {previewBanner ? (
        <div className="environment-manager__feedback environment-manager__feedback--warning">{previewBanner}</div>
      ) : null}

      {feedbackMessage ? <div className="environment-manager__feedback">{feedbackMessage}</div> : null}
      {errorMessage ? <div className="environment-manager__feedback environment-manager__feedback--error">{errorMessage}</div> : null}
      {degradedMessage ? (
        <div className="environment-manager__feedback environment-manager__feedback--warning">
          Secret store degraded. {degradedMessage}
        </div>
      ) : null}

      <div className="environment-manager__layout">
        <aside className="environment-panel environment-panel--list">
          <div className="environment-panel__header">
            <h2>Environments</h2>
            <span>{isLoading ? "Loading…" : `${environments.length} items`}</span>
          </div>

          <div className="environment-list" role="list">
            {environments.map((environment) => {
              const isActive = environment.id === activeEnvironment?.id;

              return (
                <button
                  key={environment.id}
                  type="button"
                  className={`environment-list__item${isActive ? " environment-list__item--active" : ""}`}
                  onClick={() => setActiveEnvironmentId(environment.id)}
                >
                  <span className="environment-list__name-row">
                    <strong>{environment.name}</strong>
                    {environment.isDefault ? <span className="environment-badge">Default</span> : null}
                    {environment.envType === "production" ? (
                      <span className="environment-badge environment-badge--danger">Production-like environment</span>
                    ) : null}
                  </span>
                  <span className="environment-list__meta">{getEnvironmentTypeLabel(environment.envType)}</span>
                </button>
              );
            })}

            {!isLoading && environments.length === 0 ? (
              <div className="environment-list__empty">No environments yet. Create one to start managing variables.</div>
            ) : null}
          </div>
        </aside>

        <section className="environment-panel">
          <div className="environment-panel__header">
            <h2>{environmentForm.id ? "Edit environment" : "Create environment"}</h2>
            <span>Default environment selection lives here.</span>
          </div>

          {isProductionLikeEnvironment(activeEnvironment) ? (
            <div className="environment-warning">
              <strong>Production-like environment</strong>
              <p>
                This environment is marked as production. Confirm destructive actions before deleting it or changing
                high-risk secret variables.
              </p>
            </div>
          ) : null}

          <form className="environment-form" onSubmit={(event) => void handleEnvironmentSubmit(event)}>
            <label className="environment-field">
              <span>Name</span>
              <input
                value={environmentForm.name}
                onChange={(event) =>
                  setEnvironmentForm((current) => ({
                    ...current,
                    name: event.target.value
                  }))
                }
                placeholder="e.g. Production EU"
              />
            </label>

            <label className="environment-field">
              <span>Environment type</span>
              <select
                value={environmentForm.envType}
                onChange={(event) =>
                  setEnvironmentForm((current) => ({
                    ...current,
                    envType: event.target.value as EnvironmentTypeOption
                  }))
                }
              >
                {ENVIRONMENT_TYPE_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="environment-toggle">
              <input
                type="checkbox"
                checked={environmentForm.isDefault}
                onChange={(event) =>
                  setEnvironmentForm((current) => ({
                    ...current,
                    isDefault: event.target.checked
                  }))
                }
              />
              <span>Set as default environment</span>
            </label>

            <div className="environment-form__actions">
              <button type="submit" disabled={isSavingEnvironment}>
                {isSavingEnvironment ? "Saving…" : environmentForm.id ? "Update environment" : "Create environment"}
              </button>
              <button type="button" className="environment-button--ghost" onClick={handleCreateEnvironmentDraft}>
                Reset form
              </button>
              {activeEnvironment ? (
                <button type="button" className="environment-button--danger" onClick={() => void handleDeleteEnvironment()}>
                  Delete environment
                </button>
              ) : null}
            </div>
          </form>
        </section>

        <section className="environment-panel">
          <div className="environment-panel__header">
            <h2>Variables</h2>
            <span>Secret values are masked by default.</span>
          </div>

          <form className="environment-form" onSubmit={(event) => void handleVariableSubmit(event)}>
            <label className="environment-field">
              <span>Variable key</span>
              <input
                value={variableForm.key}
                onChange={(event) =>
                  setVariableForm((current) => ({
                    ...current,
                    key: event.target.value
                  }))
                }
                placeholder="API_KEY"
              />
            </label>

            <label className="environment-field">
              <span>Variable kind</span>
              <select
                value={variableForm.kind}
                onChange={(event) =>
                  setVariableForm((current) => ({
                    ...current,
                    kind: event.target.value as VariableKindOption
                  }))
                }
              >
                {VARIABLE_KIND_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>

            <label className="environment-field">
              <span>{variableForm.kind === "secret" ? "Secret value" : "Value"}</span>
              <input
                type={variableForm.kind === "secret" ? "password" : "text"}
                value={variableForm.value}
                onChange={(event) =>
                  setVariableForm((current) => ({
                    ...current,
                    value: event.target.value
                  }))
                }
                placeholder={variableForm.kind === "secret" ? "Enter new secret" : "https://api.example.com"}
              />
            </label>

            <div className="environment-form__actions">
              <button type="submit" disabled={isSavingVariable || !activeEnvironment}>
                {isSavingVariable ? "Saving…" : variableForm.id ? "Update variable" : "Save variable"}
              </button>
              <button
                type="button"
                className="environment-button--ghost"
                onClick={() => setVariableForm(EMPTY_VARIABLE_FORM)}
              >
                Clear variable form
              </button>
            </div>
          </form>

          <div className="environment-variable-list">
            {activeEnvironment?.variables.map((variable) => (
              <article key={variable.id} className="environment-variable-card">
                <div>
                  <div className="environment-variable-card__title-row">
                    <strong>{variable.key}</strong>
                    <span className={`environment-badge${variable.kind === "secret" ? " environment-badge--warning" : ""}`}>
                      {getVariableKindLabel(variable.kind)}
                    </span>
                  </div>
                  <p>{variable.kind === "secret" ? `${variable.valueMaskedPreview} (masked by default)` : variable.valueMaskedPreview}</p>
                </div>
                <div className="environment-variable-card__actions">
                  <button type="button" className="environment-button--ghost" onClick={() => handleEditVariable(variable)}>
                    Edit variable
                  </button>
                  <button type="button" className="environment-button--danger" onClick={() => void handleDeleteVariable(variable)}>
                    Delete variable
                  </button>
                </div>
              </article>
            ))}

            {activeEnvironment && activeEnvironment.variables.length === 0 ? (
              <div className="environment-list__empty">No variables yet for this environment.</div>
            ) : null}

            {!activeEnvironment ? (
              <div className="environment-list__empty">Select or create an environment to manage variables.</div>
            ) : null}
          </div>
        </section>
      </div>
    </section>
  );
}
