import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent, FormEvent } from "react";
import type { ReactElement } from "react";
import {
  apiTesterClient,
  getApiTesterPreviewBanner,
  getApiTesterWorkspaceBanner
} from "../services/api-tester-client";
import { useEnvStore } from "../store/env-store";
import type {
  ApiAssertionDto,
  ApiAssertionResultDto,
  ApiExecutionResultDto,
  ApiRequestDto,
  ApiTestCaseDto,
  AssertionOperator
} from "../types";

type RequestMethod = ApiRequestDto["method"];
type AuthType = NonNullable<ApiRequestDto["auth"]>["type"];
type AuthLocation = NonNullable<ApiRequestDto["auth"]>["location"];
type BuilderTab = "request" | "auth" | "headers" | "body";

interface RequestFormState {
  method: RequestMethod;
  url: string;
  headersText: string;
  queryParamsText: string;
  body: string;
  authType: AuthType;
  authLocation: AuthLocation;
  authKey: string;
  authValue: string;
  authToken: string;
  authUsername: string;
  authPassword: string;
}

interface AssertionDraft {
  id: string;
  operator: AssertionOperator;
  expectedValue: string;
  sourcePath: string;
}

const REQUEST_METHOD_OPTIONS: RequestMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const ASSERTION_OPERATOR_OPTIONS: Array<{ value: AssertionOperator; label: string; description: string }> = [
  { value: "status_equals", label: "Status equals", description: "Compare the HTTP status code." },
  { value: "json_path_exists", label: "JSON path exists", description: "Assert that a JSON path resolves to a value." },
  { value: "json_path_equals", label: "JSON path equals", description: "Compare the resolved JSON path value." },
  { value: "body_contains", label: "Body contains", description: "Check a plain-text body fragment." },
  { value: "header_equals", label: "Header equals", description: "Compare a response header value." }
];
const BUILDER_TABS: Array<{ id: BuilderTab; label: string }> = [
  { id: "request", label: "Request" },
  { id: "auth", label: "Auth" },
  { id: "headers", label: "Headers" },
  { id: "body", label: "Body" }
];

const EMPTY_REQUEST_FORM: RequestFormState = {
  method: "GET",
  url: "",
  headersText: "Accept: application/json",
  queryParamsText: "",
  body: "",
  authType: "none",
  authLocation: "header",
  authKey: "X-API-Key",
  authValue: "",
  authToken: "",
  authUsername: "",
  authPassword: ""
};

function createId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function createDefaultAssertionDraft(): AssertionDraft {
  return {
    id: createId("assertion"),
    operator: "status_equals",
    expectedValue: "200",
    sourcePath: ""
  };
}

function createEmptyApiTestCase(): ApiTestCaseDto {
  return {
    id: createId("api-testcase"),
    type: "api",
    name: "Untitled request",
    request: {
      method: "GET",
      url: "",
      headers: {
        Accept: "application/json"
      },
      queryParams: {}
    },
    assertions: [
      {
        id: createId("assertion"),
        operator: "status_equals",
        expectedValue: "200"
      }
    ]
  };
}

function parseKeyValueText(value: string): Record<string, string> {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .reduce<Record<string, string>>((result, line) => {
      const separatorIndex = line.indexOf(":") >= 0 ? line.indexOf(":") : line.indexOf("=");
      if (separatorIndex < 0) {
        return result;
      }

      const key = line.slice(0, separatorIndex).trim();
      const parsedValue = line.slice(separatorIndex + 1).trim();
      if (key.length > 0) {
        result[key] = parsedValue;
      }
      return result;
    }, {});
}

function stringifyKeyValueMap(values: Record<string, string>): string {
  return Object.entries(values)
    .map(([key, value]) => `${key}: ${value}`)
    .join("\n");
}

function toAssertionDrafts(assertions: ApiAssertionDto[]): AssertionDraft[] {
  if (assertions.length === 0) {
    return [createDefaultAssertionDraft()];
  }

  return assertions.map((assertion) => ({
    id: assertion.id,
    operator: assertion.operator,
    expectedValue: assertion.expectedValue,
    sourcePath: assertion.sourcePath ?? ""
  }));
}

function toRequestFormState(request: ApiRequestDto): RequestFormState {
  return {
    method: request.method,
    url: request.url,
    headersText: stringifyKeyValueMap(request.headers),
    queryParamsText: stringifyKeyValueMap(request.queryParams),
    body: request.body ?? "",
    authType: request.auth?.type ?? "none",
    authLocation: request.auth?.location ?? "header",
    authKey: request.auth?.key ?? "X-API-Key",
    authValue: request.auth?.value ?? "",
    authToken: request.auth?.token ?? "",
    authUsername: request.auth?.username ?? "",
    authPassword: request.auth?.password ?? ""
  };
}

function toRequestDto(form: RequestFormState): ApiRequestDto {
  const headers = parseKeyValueText(form.headersText);
  const queryParams = parseKeyValueText(form.queryParamsText);
  const request: ApiRequestDto = {
    method: form.method,
    url: form.url.trim(),
    headers,
    queryParams
  };

  if (form.body.trim().length > 0) {
    request.body = form.body;
  }

  if (form.authType === "bearer") {
    request.auth = {
      type: "bearer",
      token: form.authToken
    };
  }

  if (form.authType === "basic") {
    request.auth = {
      type: "basic",
      username: form.authUsername,
      password: form.authPassword
    };
  }

  if (form.authType === "api_key") {
    request.auth = {
      type: "api_key",
      location: form.authLocation ?? "header",
      key: form.authKey,
      value: form.authValue
    };
  }

  return request;
}

function toAssertionDtos(drafts: AssertionDraft[]): ApiAssertionDto[] {
  return drafts.map((draft) => ({
    id: draft.id,
    operator: draft.operator,
    expectedValue: draft.expectedValue,
    ...(draft.sourcePath.trim().length > 0 ? { sourcePath: draft.sourcePath.trim() } : {})
  }));
}

function getAssertionOperatorLabel(operator: AssertionOperator): string {
  return ASSERTION_OPERATOR_OPTIONS.find((option) => option.value === operator)?.label ?? operator;
}

function getFailureTone(result: ApiExecutionResultDto | null): { title: string; description: string; modifier: string } {
  if (!result) {
    return {
      title: "No result yet",
      description: "Run request to populate the result state panel.",
      modifier: ""
    };
  }

  if (result.status === "passed") {
    return {
      title: "Assertions passed",
      description: "Transport succeeded and all assertions matched expected values.",
      modifier: " api-tester__result-summary--success"
    };
  }

  if (result.failureKind === "preflight") {
    return {
      title: "Preflight failed",
      description: "Request build failed before dispatch, usually because a variable or request shape is invalid.",
      modifier: " api-tester__result-summary--warning"
    };
  }

  if (result.failureKind === "transport") {
    return {
      title: "Transport failed",
      description: "The request could not reach the endpoint or did not return a usable response.",
      modifier: " api-tester__result-summary--error"
    };
  }

  return {
    title: "Assertion failed",
    description: "Transport completed, but at least one expected value did not match the actual response.",
    modifier: " api-tester__result-summary--error"
  };
}

function getAssertionStats(result: ApiExecutionResultDto | null): { passed: number; failed: number; total: number } {
  const assertions = result?.assertions ?? [];
  const passed = assertions.filter((assertion) => assertion.passed).length;
  return {
    passed,
    failed: assertions.length - passed,
    total: assertions.length
  };
}

function createResponseHeaderLines(headers: Record<string, string>): string[] {
  return Object.entries(headers).map(([key, value]) => `${key}: ${value}`);
}

function createCollectionGroups(testCases: ApiTestCaseDto[]) {
  return {
    draft: testCases.filter((testCase) => !testCase.name.toLowerCase().includes("saved")),
    saved: testCases.filter((testCase) => testCase.name.toLowerCase().includes("saved"))
  };
}

export default function ApiTester() {
  const { activeEnvironmentId, environments } = useEnvStore();
  const [testCases, setTestCases] = useState<ApiTestCaseDto[]>([]);
  const [activeTestCaseId, setActiveTestCaseId] = useState<string | null>(null);
  const [name, setName] = useState("Untitled request");
  const [requestForm, setRequestForm] = useState<RequestFormState>(EMPTY_REQUEST_FORM);
  const [assertionDrafts, setAssertionDrafts] = useState<AssertionDraft[]>([createDefaultAssertionDraft()]);
  const [builderTab, setBuilderTab] = useState<BuilderTab>("request");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [executionResult, setExecutionResult] = useState<ApiExecutionResultDto | null>(null);

  const previewBanner = getApiTesterPreviewBanner();
  const workspaceBanner = getApiTesterWorkspaceBanner();
  const activeEnvironment = environments.find((environment) => environment.id === activeEnvironmentId) ?? null;
  const activeTestCase = useMemo(
    () => testCases.find((testCase) => testCase.id === activeTestCaseId) ?? null,
    [activeTestCaseId, testCases]
  );
  const collectionGroups = useMemo(() => createCollectionGroups(testCases), [testCases]);
  const requestPreview = executionResult?.requestPreview ?? null;
  const failureTone = getFailureTone(executionResult);
  const assertionStats = getAssertionStats(executionResult);

  useEffect(() => {
    void loadWorkspace();
  }, []);

  useEffect(() => {
    if (!activeTestCase) {
      return;
    }

    hydrateEditor(activeTestCase);
  }, [activeTestCase]);

  async function loadWorkspace(): Promise<void> {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const records = await apiTesterClient.loadWorkspace();
      setTestCases(records);
      const selected = records[0] ?? null;
      setActiveTestCaseId(selected?.id ?? null);

      if (selected) {
        hydrateEditor(selected);
      } else {
        const draft = createEmptyApiTestCase();
        hydrateEditor(draft);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể tải API Tester workspace.";
      setErrorMessage(message);
    } finally {
      setIsLoading(false);
    }
  }

  function hydrateEditor(testCase: ApiTestCaseDto): void {
    setName(testCase.name);
    setRequestForm(toRequestFormState(testCase.request));
    setAssertionDrafts(toAssertionDrafts(testCase.assertions));
  }

  function handleNewDraft(): void {
    const draft = createEmptyApiTestCase();
    setActiveTestCaseId(null);
    setExecutionResult(null);
    setFeedbackMessage("Empty state ready. Start authoring a new API test.");
    setErrorMessage(null);
    hydrateEditor(draft);
  }

  function handleRequestFormChange<Key extends keyof RequestFormState>(key: Key, value: RequestFormState[Key]): void {
    setRequestForm((current) => ({
      ...current,
      [key]: value
    }));
  }

  function handleAssertionChange(index: number, patch: Partial<AssertionDraft>): void {
    setAssertionDrafts((current) =>
      current.map((draft, draftIndex) => (draftIndex === index ? { ...draft, ...patch } : draft))
    );
  }

  function handleAddAssertion(): void {
    setAssertionDrafts((current) => [...current, createDefaultAssertionDraft()]);
  }

  function handleRemoveAssertion(assertionId: string): void {
    setAssertionDrafts((current) => {
      if (current.length === 1) {
        return current;
      }

      return current.filter((assertion) => assertion.id !== assertionId);
    });
  }

  function buildEditableTestCase(): ApiTestCaseDto {
    return {
      id: activeTestCaseId ?? createId("api-testcase"),
      type: "api",
      name: name.trim() || "Untitled request",
      request: toRequestDto(requestForm),
      assertions: toAssertionDtos(assertionDrafts)
    };
  }

  async function handleSave(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    const draft = buildEditableTestCase();
    if (draft.request.url.trim().length === 0) {
      setErrorMessage("Request URL is required before saving.");
      return;
    }

    setIsSaving(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      const saved = await apiTesterClient.upsert(draft);
      setTestCases((current) => {
        const existingIndex = current.findIndex((testCase) => testCase.id === saved.id);
        const next = [...current];
        if (existingIndex >= 0) {
          next.splice(existingIndex, 1, saved);
        } else {
          next.unshift(saved);
        }
        return next;
      });
      setActiveTestCaseId(saved.id);
      setFeedbackMessage("API test case saved.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu API test case.";
      setErrorMessage(message);
    } finally {
      setIsSaving(false);
    }
  }

  async function handleDelete(): Promise<void> {
    if (!activeTestCaseId || !activeTestCase) {
      return;
    }

    if (!window.confirm(`Delete API test case '${activeTestCase.name}'?`)) {
      return;
    }

    setIsDeleting(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      await apiTesterClient.delete(activeTestCaseId);
      const remaining = testCases.filter((testCase) => testCase.id !== activeTestCaseId);
      setTestCases(remaining);
      setActiveTestCaseId(remaining[0]?.id ?? null);
      setExecutionResult(null);
      if (remaining[0]) {
        hydrateEditor(remaining[0]);
      } else {
        hydrateEditor(createEmptyApiTestCase());
      }
      setFeedbackMessage("API test case deleted.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa API test case.";
      setErrorMessage(message);
    } finally {
      setIsDeleting(false);
    }
  }

  async function handleRun(): Promise<void> {
    const draft = buildEditableTestCase();
    if (!activeEnvironmentId) {
      setErrorMessage("Select an environment before running the API test.");
      return;
    }

    if (draft.request.url.trim().length === 0) {
      setErrorMessage("Request URL is required before execution.");
      return;
    }

    setIsRunning(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      const executeInput = {
        environmentId: activeEnvironmentId,
        request: draft.request,
        assertions: draft.assertions
      };
      const result = await apiTesterClient.execute(
        activeTestCaseId
          ? {
              ...executeInput,
              testCaseId: activeTestCaseId
            }
          : executeInput
      );
      setExecutionResult(result);
      setFeedbackMessage(result.status === "passed" ? "Request passed." : "Request completed with failures.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể chạy API test.";
      setErrorMessage(message);
    } finally {
      setIsRunning(false);
    }
  }

  function renderBuilderTab(): ReactElement {
    if (builderTab === "request") {
      return (
        <div className="api-tester__tab-panel" aria-label="Request builder">
          <label className="api-tester__field">
            <span>Request name</span>
            <input value={name} onChange={(event) => setName(event.target.value)} placeholder="Saved request name" />
          </label>
          <div className="api-tester__field-row">
            <label className="api-tester__field api-tester__field--compact">
              <span>Method</span>
              <select
                value={requestForm.method}
                onChange={(event) => handleRequestFormChange("method", event.target.value as RequestMethod)}
              >
                {REQUEST_METHOD_OPTIONS.map((method) => (
                  <option key={method} value={method}>
                    {method}
                  </option>
                ))}
              </select>
            </label>

            <label className="api-tester__field api-tester__field--wide">
              <span>URL</span>
              <input
                value={requestForm.url}
                onChange={(event) => handleRequestFormChange("url", event.target.value)}
                placeholder="https://api.example.com/users"
              />
            </label>
          </div>

          <label className="api-tester__field">
            <span>Query params</span>
            <textarea
              value={requestForm.queryParamsText}
              onChange={(event) => handleRequestFormChange("queryParamsText", event.target.value)}
              rows={4}
              placeholder="page: 1\nlimit: 25"
            />
          </label>
        </div>
      );
    }

    if (builderTab === "auth") {
      return (
        <div className="api-tester__tab-panel" aria-label="Auth builder">
          <label className="api-tester__field api-tester__field--compact">
            <span>Auth type</span>
            <select
              value={requestForm.authType}
              onChange={(event) => handleRequestFormChange("authType", event.target.value as AuthType)}
            >
              <option value="none">None</option>
              <option value="bearer">Bearer</option>
              <option value="basic">Basic</option>
              <option value="api_key">API key</option>
            </select>
          </label>

          {requestForm.authType === "bearer" ? (
            <label className="api-tester__field">
              <span>Bearer token</span>
              <input
                value={requestForm.authToken}
                onChange={(event) => handleRequestFormChange("authToken", event.target.value)}
                placeholder="Will appear as [REDACTED] in request preview"
              />
            </label>
          ) : null}

          {requestForm.authType === "basic" ? (
            <div className="api-tester__field-row">
              <label className="api-tester__field api-tester__field--wide">
                <span>Username</span>
                <input
                  value={requestForm.authUsername}
                  onChange={(event) => handleRequestFormChange("authUsername", event.target.value)}
                />
              </label>
              <label className="api-tester__field api-tester__field--wide">
                <span>Password</span>
                <input
                  value={requestForm.authPassword}
                  onChange={(event) => handleRequestFormChange("authPassword", event.target.value)}
                />
              </label>
            </div>
          ) : null}

          {requestForm.authType === "api_key" ? (
            <>
              <div className="api-tester__field-row">
                <label className="api-tester__field api-tester__field--compact">
                  <span>Location</span>
                  <select
                    value={requestForm.authLocation}
                    onChange={(event) => handleRequestFormChange("authLocation", event.target.value as AuthLocation)}
                  >
                    <option value="header">Header</option>
                    <option value="query">Query</option>
                  </select>
                </label>
                <label className="api-tester__field api-tester__field--wide">
                  <span>Key</span>
                  <input
                    value={requestForm.authKey}
                    onChange={(event) => handleRequestFormChange("authKey", event.target.value)}
                  />
                </label>
              </div>
              <label className="api-tester__field">
                <span>Value</span>
                <input
                  value={requestForm.authValue}
                  onChange={(event) => handleRequestFormChange("authValue", event.target.value)}
                  placeholder="Will appear as [REDACTED] in request preview"
                />
              </label>
            </>
          ) : null}

          {requestForm.authType === "none" ? (
            <div className="api-tester__empty-panel">Empty state · No auth configured for this request.</div>
          ) : null}
        </div>
      );
    }

    if (builderTab === "headers") {
      return (
        <div className="api-tester__tab-panel" aria-label="Headers builder">
          <label className="api-tester__field">
            <span>Headers</span>
            <textarea
              value={requestForm.headersText}
              onChange={(event) => handleRequestFormChange("headersText", event.target.value)}
              rows={8}
              placeholder="Accept: application/json\nAuthorization: [REDACTED]"
            />
          </label>
          <p className="api-tester__hint">Sensitive headers are always redacted inside request previews.</p>
        </div>
      );
    }

    return (
      <div className="api-tester__tab-panel" aria-label="Body builder">
        <label className="api-tester__field">
          <span>Body</span>
          <textarea
            value={requestForm.body}
            onChange={(event) => handleRequestFormChange("body", event.target.value)}
            rows={12}
            placeholder='{"name":"QA"}'
          />
        </label>
        <p className="api-tester__hint">Response viewer shows a bounded body preview so QA can inspect payload shape safely.</p>
      </div>
    );
  }

  function renderAssertionResult(assertion: ApiAssertionResultDto): ReactElement {
    return (
      <article
        key={assertion.assertionId}
        className={`api-tester__assertion-result${assertion.passed ? " api-tester__assertion-result--passed" : " api-tester__assertion-result--failed"}`}
      >
        <div className="api-tester__assertion-result-header">
          <strong>{getAssertionOperatorLabel(assertion.operator)}</strong>
          <span>{assertion.passed ? "Passed" : "Failed"}</span>
        </div>
        <dl className="api-tester__comparison-grid">
          <div>
            <dt>expected</dt>
            <dd>{assertion.expectedValue || "—"}</dd>
          </div>
          <div>
            <dt>actual</dt>
            <dd>{assertion.actualValue ?? "—"}</dd>
          </div>
        </dl>
        {assertion.sourcePath ? <p>Source path: {assertion.sourcePath}</p> : null}
        {assertion.message ? <p>{assertion.message}</p> : null}
      </article>
    );
  }

  return (
    <section className="api-tester" data-testid="route-api-tester">
      <header className="api-tester__hero">
        <div>
          <span className="route-skeleton__eyebrow">API Tester</span>
          <h1>Author requests, define assertions, and inspect actual vs expected API behavior</h1>
          <p>
            Collection tree, request builder, redacted request preview, and result viewer all stay on the typed T8
            surface. Transport, preflight, and assertion failures are separated explicitly for QA clarity.
          </p>
        </div>

        <div className="api-tester__hero-actions">
          <button type="button" className="api-tester__primary-action" onClick={handleNewDraft}>
            New request
          </button>
          <button type="button" className="api-tester__secondary-action" onClick={() => void handleRun()} disabled={isRunning}>
            {isRunning ? "Running…" : "Run request"}
          </button>
        </div>
      </header>

      {previewBanner ? <div className="api-tester__feedback">{previewBanner}</div> : null}
      {workspaceBanner ? <div className="api-tester__feedback api-tester__feedback--warning">{workspaceBanner}</div> : null}
      {feedbackMessage ? <div className="api-tester__feedback">{feedbackMessage}</div> : null}
      {errorMessage ? <div className="api-tester__feedback api-tester__feedback--error">{errorMessage}</div> : null}

      <div className="api-tester__layout">
        <aside className="api-tester__panel api-tester__panel--collection">
          <div className="api-tester__panel-header">
            <div>
              <span className="api-tester__eyebrow">Collection</span>
              <h2>Collection tree</h2>
            </div>
            <span>{isLoading ? "Loading…" : `${testCases.length} items`}</span>
          </div>

          {isLoading ? <div className="api-tester__empty-panel">Loading state · Loading saved requests…</div> : null}

          {!isLoading && testCases.length === 0 ? (
            <div className="api-tester__empty-panel">
              Empty state · No saved API tests yet. Save the current request to populate the collection tree.
            </div>
          ) : null}

          {!isLoading ? (
            <div className="api-tester__collection-groups">
              <div className="api-tester__collection-group">
                <div className="api-tester__collection-label">Draft</div>
                {collectionGroups.draft.map((testCase) => (
                  <button
                    key={testCase.id}
                    type="button"
                    className={`api-tester__collection-item${testCase.id === activeTestCaseId ? " api-tester__collection-item--active" : ""}`}
                    onClick={() => setActiveTestCaseId(testCase.id)}
                  >
                    <strong>{testCase.name}</strong>
                    <span>{testCase.request.method} · {testCase.request.url || "URL missing"}</span>
                  </button>
                ))}
              </div>

              <div className="api-tester__collection-group">
                <div className="api-tester__collection-label">Saved</div>
                {collectionGroups.saved.length === 0 ? <div className="api-tester__collection-placeholder">No Saved collection items yet.</div> : null}
                {collectionGroups.saved.map((testCase) => (
                  <button
                    key={testCase.id}
                    type="button"
                    className={`api-tester__collection-item${testCase.id === activeTestCaseId ? " api-tester__collection-item--active" : ""}`}
                    onClick={() => setActiveTestCaseId(testCase.id)}
                  >
                    <strong>{testCase.name}</strong>
                    <span>{testCase.assertions.length} assertions</span>
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        </aside>

        <form className="api-tester__panel api-tester__panel--builder" onSubmit={(event) => void handleSave(event)}>
          <div className="api-tester__panel-header">
            <div>
              <span className="api-tester__eyebrow">Builder</span>
              <h2>Request builder</h2>
            </div>
            <span>{activeEnvironment ? `Environment · ${activeEnvironment.name}` : "Select environment in Environment Manager"}</span>
          </div>

          <div className="api-tester__tab-list" role="tablist" aria-label="API builder tabs">
            {BUILDER_TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                role="tab"
                aria-selected={builderTab === tab.id}
                className={`api-tester__tab${builderTab === tab.id ? " api-tester__tab--active" : ""}`}
                onClick={() => setBuilderTab(tab.id)}
              >
                {tab.label}
              </button>
            ))}
          </div>

          {renderBuilderTab()}

          <section className="api-tester__assertion-builder" aria-label="Assertions">
            <div className="api-tester__subsection-header">
              <div>
                <span className="api-tester__eyebrow">Assertions</span>
                <h3>Assertion builder UI</h3>
              </div>
              <button type="button" className="api-tester__tertiary-action" onClick={handleAddAssertion}>
                Add assertion
              </button>
            </div>

            <div className="api-tester__assertion-list">
              {assertionDrafts.map((assertion, index) => {
                const operatorMeta = ASSERTION_OPERATOR_OPTIONS.find((option) => option.value === assertion.operator);

                return (
                  <article key={assertion.id} className="api-tester__assertion-card">
                    <div className="api-tester__assertion-card-header">
                      <strong>Assertion {index + 1}</strong>
                      <button
                        type="button"
                        className="api-tester__ghost-action"
                        onClick={() => handleRemoveAssertion(assertion.id)}
                        disabled={assertionDrafts.length === 1}
                      >
                        Remove
                      </button>
                    </div>

                    <div className="api-tester__field-row">
                      <label className="api-tester__field api-tester__field--wide">
                        <span>Operator</span>
                        <select
                          value={assertion.operator}
                          onChange={(event) =>
                            handleAssertionChange(index, { operator: event.target.value as AssertionOperator })
                          }
                        >
                          {ASSERTION_OPERATOR_OPTIONS.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="api-tester__field api-tester__field--wide">
                        <span>Expected value</span>
                        <input
                          value={assertion.expectedValue}
                          onChange={(event) => handleAssertionChange(index, { expectedValue: event.target.value })}
                          placeholder="200"
                        />
                      </label>
                    </div>

                    <label className="api-tester__field">
                      <span>Source path</span>
                      <input
                        value={assertion.sourcePath}
                        onChange={(event) => handleAssertionChange(index, { sourcePath: event.target.value })}
                        placeholder="$.data.0.id or content-type"
                      />
                    </label>

                    <p className="api-tester__hint">{operatorMeta?.description ?? "Assertion"}</p>
                  </article>
                );
              })}
            </div>
          </section>

          <div className="api-tester__form-actions">
            <button type="submit" className="api-tester__primary-action" disabled={isSaving}>
              {isSaving ? "Saving…" : "Save request"}
            </button>
            <button type="button" className="api-tester__secondary-action" onClick={() => void handleRun()} disabled={isRunning}>
              {isRunning ? "Running…" : "Run request"}
            </button>
            <button
              type="button"
              className="api-tester__danger-action"
              onClick={() => void handleDelete()}
              disabled={!activeTestCaseId || isDeleting}
            >
              {isDeleting ? "Deleting…" : "Delete request"}
            </button>
          </div>
        </form>

        <section className="api-tester__panel api-tester__panel--results">
          <div className="api-tester__panel-header">
            <div>
              <span className="api-tester__eyebrow">Response viewer</span>
              <h2>Result state panels</h2>
            </div>
            <span>{executionResult?.durationMs ? `${executionResult.durationMs} ms` : "Waiting for first run"}</span>
          </div>

          {isRunning ? <div className="api-tester__empty-panel">Loading state · Running request and waiting for result…</div> : null}

          {!isRunning && !executionResult ? (
            <div className="api-tester__empty-panel">
              Empty state · Response viewer, assertion summary, and actual vs expected details appear after a run.
            </div>
          ) : null}

          {executionResult ? (
            <>
              <article className={`api-tester__result-summary${failureTone.modifier}`}>
                <div>
                  <h3>{failureTone.title}</h3>
                  <p>{failureTone.description}</p>
                </div>
                <dl className="api-tester__metric-grid">
                  <div>
                    <dt>Status</dt>
                    <dd>{executionResult.status}</dd>
                  </div>
                  <div>
                    <dt>transport</dt>
                    <dd>{executionResult.transportSuccess ? "success" : "failed"}</dd>
                  </div>
                  <div>
                    <dt>failureKind</dt>
                    <dd>{executionResult.failureKind ?? "none"}</dd>
                  </div>
                  <div>
                    <dt>statusCode</dt>
                    <dd>{executionResult.statusCode ?? "—"}</dd>
                  </div>
                </dl>
              </article>

              <section className="api-tester__subpanel">
                <div className="api-tester__subsection-header">
                  <div>
                    <span className="api-tester__eyebrow">Request preview</span>
                    <h3>Redacted request preview</h3>
                  </div>
                  <span>{requestPreview?.method} · {requestPreview?.url}</span>
                </div>

                {requestPreview ? (
                  <>
                    <dl className="api-tester__request-grid">
                      <div>
                        <dt>Auth</dt>
                        <dd>{requestPreview.authPreview}</dd>
                      </div>
                      <div>
                        <dt>Headers</dt>
                        <dd>{Object.keys(requestPreview.headers).length}</dd>
                      </div>
                      <div>
                        <dt>Query params</dt>
                        <dd>{Object.keys(requestPreview.queryParams).length}</dd>
                      </div>
                    </dl>

                    <div className="api-tester__code-grid">
                      <div>
                        <h4>Headers</h4>
                        <pre>{JSON.stringify(requestPreview.headers, null, 2)}</pre>
                      </div>
                      <div>
                        <h4>Query params</h4>
                        <pre>{JSON.stringify(requestPreview.queryParams, null, 2)}</pre>
                      </div>
                    </div>

                    <div className="api-tester__code-block">
                      <h4>Body preview</h4>
                      <pre>{requestPreview.bodyPreview ?? "No request body"}</pre>
                    </div>
                  </>
                ) : null}
              </section>

              <section className="api-tester__subpanel">
                <div className="api-tester__subsection-header">
                  <div>
                    <span className="api-tester__eyebrow">Assertions</span>
                    <h3>Assertion summary</h3>
                  </div>
                  <span>
                    {assertionStats.passed}/{assertionStats.total} passed
                  </span>
                </div>

                <div className="api-tester__metric-grid api-tester__metric-grid--three">
                  <div>
                    <dt>total</dt>
                    <dd>{assertionStats.total}</dd>
                  </div>
                  <div>
                    <dt>passed</dt>
                    <dd>{assertionStats.passed}</dd>
                  </div>
                  <div>
                    <dt>failed</dt>
                    <dd>{assertionStats.failed}</dd>
                  </div>
                </div>

                <div className="api-tester__assertion-results">{executionResult.assertions.map(renderAssertionResult)}</div>
              </section>

              <section className="api-tester__subpanel">
                <div className="api-tester__subsection-header">
                  <div>
                    <span className="api-tester__eyebrow">Response viewer</span>
                    <h3>Response viewer</h3>
                  </div>
                  <span>{executionResult.errorCode ?? "No explicit error code"}</span>
                </div>

                {executionResult.errorMessage ? (
                  <div className="api-tester__feedback api-tester__feedback--warning">{executionResult.errorMessage}</div>
                ) : null}

                <div className="api-tester__code-grid">
                  <div>
                    <h4>Response headers</h4>
                    <pre>{createResponseHeaderLines(executionResult.responseHeaders).join("\n") || "No response headers"}</pre>
                  </div>
                  <div>
                    <h4>Body preview</h4>
                    <pre>{executionResult.bodyPreview || "No response body"}</pre>
                  </div>
                </div>
              </section>
            </>
          ) : null}
        </section>
      </div>
    </section>
  );
}
