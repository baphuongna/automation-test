import { useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import { environmentClient } from "../services/environment-client";
import { runnerClient } from "../services/runner-client";
import { useEnvStore } from "../store/env-store";
import { subscribeRunnerEvents, useRunStore } from "../store/run-store";
import type { RunCaseResultDto, RunDetailDto, RunHistoryEntryDto, SuiteDto } from "../types";

function formatRunStatus(status: RunHistoryEntryDto["status"]): string {
  switch (status) {
    case "passed":
      return "Passed";
    case "failed":
      return "Failed";
    case "running":
      return "Running";
    case "queued":
      return "Queued";
    case "cancelled":
      return "Cancelled";
    case "skipped":
      return "Skipped";
    default:
      return "Idle";
  }
}

function formatTimestamp(value: string | undefined): string {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function summarizeAssertions(result: RunCaseResultDto): string {
  if (result.assertionPreview.trim().length === 0 || result.assertionPreview.trim() === "[]") {
    return "No assertion detail captured.";
  }

  return result.assertionPreview;
}

function buildProgressPercent(completed: number, total: number): number {
  if (total <= 0) {
    return 0;
  }

  return Math.min(100, Math.round((completed / total) * 100));
}

function getEnvironmentLabel(activeEnvironmentId: string | null, suiteFilterId: string | null): string {
  if (!activeEnvironmentId) {
    return suiteFilterId ? "Select environment to run filtered suite" : "Select environment to run suite";
  }

  return `Environment ready · ${activeEnvironmentId}`;
}

export default function TestRunner(): ReactElement {
  const [suites, setSuites] = useState<SuiteDto[]>([]);
  const [history, setHistory] = useState<RunHistoryEntryDto[]>([]);
  const [selectedSuiteId, setSelectedSuiteId] = useState<string | null>(null);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetailDto | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [isRerunningFailed, setIsRerunningFailed] = useState(false);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const activeEnvironmentId = useEnvStore((state) => state.activeEnvironmentId);
  const setEnvironments = useEnvStore((state) => state.setEnvironments);
  const environments = useEnvStore((state) => state.environments);
  const activeRunId = useRunStore((state) => state.activeRunId);
  const progress = useRunStore((state) => state.progress);
  const runStatus = useRunStore((state) => state.status);

  const selectedSuite = useMemo(
    () => suites.find((suite) => suite.id === selectedSuiteId) ?? null,
    [selectedSuiteId, suites]
  );

  const selectedRun = useMemo(
    () => history.find((entry) => entry.runId === selectedRunId) ?? null,
    [history, selectedRunId]
  );

  const activeEnvironment = useMemo(
    () => environments.find((environment) => environment.id === activeEnvironmentId) ?? null,
    [activeEnvironmentId, environments]
  );

  async function loadRunnerScreen(options: { preserveSelectedRun?: boolean } = {}): Promise<void> {
    const preserveSelectedRun = options.preserveSelectedRun ?? false;

    setErrorMessage(null);

    const [suiteItems, environmentItems, historyItems] = await Promise.all([
      runnerClient.listSuites(),
      environmentClient.list(),
      runnerClient.listRunHistory(selectedSuiteId ? { suiteId: selectedSuiteId } : {})
    ]);

    setSuites(suiteItems);
    setEnvironments(
      environmentItems.map((environment) => ({
        id: environment.id,
        name: environment.name,
        envType: environment.envType,
        isDefault: environment.isDefault
      }))
    );
    setHistory(historyItems);

    const nextSelectedRunId = preserveSelectedRun ? selectedRunId : historyItems[0]?.runId ?? null;
    setSelectedRunId(nextSelectedRunId);

    if (nextSelectedRunId) {
      const detail = await runnerClient.getRunDetail({ runId: nextSelectedRunId });
      setRunDetail(detail);
    } else {
      setRunDetail(null);
    }
  }

  useEffect(() => {
    let isMounted = true;

    void (async () => {
      try {
        setIsLoading(true);
        await loadRunnerScreen();
      } catch (error) {
        if (isMounted) {
          setErrorMessage(error instanceof Error ? error.message : "Không thể tải runner screen.");
        }
      } finally {
        if (isMounted) {
          setIsLoading(false);
        }
      }
    })();

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    if (isLoading) {
      return;
    }

    void handleRefresh();
  }, [selectedSuiteId]);

  useEffect(() => {
    const unsubscribe = subscribeRunnerEvents({
      onStarted: (payload) => {
        setFeedbackMessage(`Suite queued · run ${payload.runId} is now active.`);
      },
      onProgress: (payload) => {
        setFeedbackMessage(
          `Live progress · ${payload.completedCount}/${payload.totalCount} complete, ${payload.failedCount} failed.`
        );
      },
      onCompleted: (payload) => {
        setFeedbackMessage(`Run completed · ${formatRunStatus(payload.status)} with ${payload.failedCount} failed result(s).`);
        setSelectedRunId(payload.runId);
        void refreshAfterRun(payload.runId);
      }
    });

    return unsubscribe;
  }, [selectedSuiteId]);

  useEffect(() => {
    if (!selectedRunId) {
      setRunDetail(null);
      return;
    }

    let isMounted = true;

    void (async () => {
      try {
        const detail = await runnerClient.getRunDetail({ runId: selectedRunId });
        if (isMounted) {
          setRunDetail(detail);
        }
      } catch (error) {
        if (isMounted) {
          setErrorMessage(error instanceof Error ? error.message : "Không thể tải run detail.");
        }
      }
    })();

    return () => {
      isMounted = false;
    };
  }, [selectedRunId]);

  async function refreshAfterRun(nextRunId?: string): Promise<void> {
    try {
      setIsRefreshing(true);
      await loadRunnerScreen({ preserveSelectedRun: true });
      if (nextRunId) {
        setSelectedRunId(nextRunId);
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể làm mới lịch sử chạy.");
    } finally {
      setIsRefreshing(false);
      setIsExecuting(false);
      setIsRerunningFailed(false);
      setIsCancelling(false);
    }
  }

  async function handleRefresh(): Promise<void> {
    setFeedbackMessage(null);

    try {
      setIsRefreshing(true);
      await loadRunnerScreen({ preserveSelectedRun: true });
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể refresh runner screen.");
    } finally {
      setIsRefreshing(false);
    }
  }

  async function handleRunSuite(): Promise<void> {
    if (!selectedSuite || !activeEnvironmentId) {
      setErrorMessage("Select a suite and environment before starting the runner.");
      return;
    }

    try {
      setErrorMessage(null);
      setFeedbackMessage(null);
      setIsExecuting(true);
      const result = await runnerClient.executeSuite({
        suiteId: selectedSuite.id,
        environmentId: activeEnvironmentId
      });
      setFeedbackMessage(`Runner control accepted · ${result.suite.name} started as ${result.runId}.`);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể chạy suite.");
      setIsExecuting(false);
    }
  }

  async function handleCancelSuite(): Promise<void> {
    if (!activeRunId) {
      return;
    }

    try {
      setErrorMessage(null);
      setIsCancelling(true);
      await runnerClient.cancelSuite({ runId: activeRunId });
      setFeedbackMessage(`Cancel requested for active run ${activeRunId}.`);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể cancel suite run.");
      setIsCancelling(false);
    }
  }

  async function handleRerunFailed(): Promise<void> {
    if (!selectedRun || !selectedRun.suiteId || !activeEnvironmentId) {
      setErrorMessage("Select a persisted suite run and environment before rerunning failures.");
      return;
    }

    try {
      setErrorMessage(null);
      setIsRerunningFailed(true);
      await runnerClient.executeSuite({
        suiteId: selectedRun.suiteId,
        environmentId: activeEnvironmentId,
        rerunFailedFromRunId: selectedRun.runId
      });
      setFeedbackMessage(`Rerun failed accepted from historical run ${selectedRun.runId}.`);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể rerun failed.");
      setIsRerunningFailed(false);
    }
  }

  const liveCompleted = progress?.completed ?? 0;
  const liveTotal = progress?.total ?? selectedRun?.totalCount ?? 0;
  const livePercent = buildProgressPercent(liveCompleted, liveTotal);
  const canRun = Boolean(selectedSuite && activeEnvironmentId) && !isExecuting;
  const canRerunFailed = Boolean(selectedRun?.suiteId && selectedRun.failedCount > 0 && activeEnvironmentId) && !isRerunningFailed;

  return (
    <section className="test-runner" data-testid="route-test-runner">
      <header className="test-runner__hero">
        <div>
          <span className="route-skeleton__eyebrow">Runner control</span>
          <h1>Track live suite execution, scan persisted history, and rerun only the failed targets</h1>
          <p>
            Active progress stays wired to the existing runner event/store seam, while history and run detail hydrate
            from persisted suite runs, case results, artifact links, and sanitized previews.
          </p>
        </div>

        <div className="test-runner__hero-actions">
          <button type="button" className="test-runner__primary-action" onClick={() => void handleRunSuite()} disabled={!canRun}>
            {isExecuting ? "Starting…" : "Run suite"}
          </button>
          <button
            type="button"
            className="test-runner__secondary-action"
            onClick={() => void handleRerunFailed()}
            disabled={!canRerunFailed}
          >
            {isRerunningFailed ? "Rerunning…" : "Rerun failed"}
          </button>
          <button
            type="button"
            className="test-runner__ghost-action"
            onClick={() => void handleRefresh()}
            disabled={isRefreshing}
          >
            {isRefreshing ? "Refreshing…" : "Refresh"}
          </button>
          <button
            type="button"
            className="test-runner__danger-action"
            onClick={() => void handleCancelSuite()}
            disabled={!activeRunId || isCancelling}
          >
            {isCancelling ? "Cancelling…" : "Cancel active run"}
          </button>
        </div>
      </header>

      {feedbackMessage ? <div className="test-runner__feedback">{feedbackMessage}</div> : null}
      {errorMessage ? <div className="test-runner__feedback test-runner__feedback--error">{errorMessage}</div> : null}

      <div className="test-runner__layout">
        <aside className="test-runner__panel test-runner__panel--control">
          <div className="test-runner__panel-header">
            <div>
              <span className="test-runner__eyebrow">Runner control</span>
              <h2>Suites and live progress</h2>
            </div>
            <span>{selectedSuite ? `${selectedSuite.items.length} cases` : `${suites.length} suites`}</span>
          </div>

          {isLoading ? <div className="test-runner__empty-panel">Loading state · Hydrating suites, environments, and run history…</div> : null}

          {!isLoading ? (
            <>
              <label className="test-runner__field">
                <span>Suite filter</span>
                <select
                  value={selectedSuiteId ?? ""}
                  onChange={(event) => setSelectedSuiteId(event.target.value || null)}
                >
                  <option value="">All suites</option>
                  {suites.map((suite) => (
                    <option key={suite.id} value={suite.id}>
                      {suite.name}
                    </option>
                  ))}
                </select>
              </label>

              <div className="test-runner__status-card">
                <div className="test-runner__status-row">
                  <strong>Active run</strong>
                  <span className={`test-runner__status-badge test-runner__status-badge--${runStatus}`}>{formatRunStatus(runStatus)}</span>
                </div>
                <p>{activeRunId ? `runId · ${activeRunId}` : "No active run right now."}</p>
                <p>{activeEnvironment?.name ? `Environment ready · ${activeEnvironment.name}` : getEnvironmentLabel(activeEnvironmentId, selectedSuiteId)}</p>
                <div className="test-runner__progress-track" aria-label="Active progress bar">
                  <span className="test-runner__progress-value" style={{ width: `${livePercent}%` }} />
                </div>
                <dl className="test-runner__metric-grid test-runner__metric-grid--four">
                  <div>
                    <dt>completed</dt>
                    <dd>{liveCompleted}</dd>
                  </div>
                  <div>
                    <dt>passed</dt>
                    <dd>{progress?.passed ?? 0}</dd>
                  </div>
                  <div>
                    <dt>failed</dt>
                    <dd>{progress?.failed ?? 0}</dd>
                  </div>
                  <div>
                    <dt>skipped</dt>
                    <dd>{progress?.skipped ?? 0}</dd>
                  </div>
                </dl>
                <p>{liveTotal > 0 ? `${liveCompleted}/${liveTotal} complete · ${livePercent}%` : "Waiting for first suite execution."}</p>
              </div>

              <div className="test-runner__suite-list">
                {suites.length === 0 ? <div className="test-runner__empty-panel">Empty state · No suites persisted yet.</div> : null}
                {suites.map((suite) => (
                  <button
                    key={suite.id}
                    type="button"
                    className={`test-runner__suite-item${suite.id === selectedSuiteId ? " test-runner__suite-item--active" : ""}`}
                    onClick={() => setSelectedSuiteId((current) => (current === suite.id ? null : suite.id))}
                  >
                    <strong>{suite.name}</strong>
                    <span>{suite.items.length} ordered cases</span>
                  </button>
                ))}
              </div>
            </>
          ) : null}
        </aside>

        <section className="test-runner__panel test-runner__panel--history">
          <div className="test-runner__panel-header">
            <div>
              <span className="test-runner__eyebrow">Run history</span>
              <h2>Persisted suite execution history</h2>
            </div>
            <span>{history.length} runs</span>
          </div>

          {isLoading ? <div className="test-runner__empty-panel">Loading state · Fetching persisted runs…</div> : null}

          {!isLoading && history.length === 0 ? (
            <div className="test-runner__empty-panel">Empty state · No persisted runner history matches the current filter.</div>
          ) : null}

          {!isLoading ? (
            <div className="test-runner__history-list">
              {history.map((entry) => (
                <button
                  key={entry.runId}
                  type="button"
                  className={`test-runner__history-item${entry.runId === selectedRunId ? " test-runner__history-item--active" : ""}`}
                  onClick={() => setSelectedRunId(entry.runId)}
                >
                  <div className="test-runner__status-row">
                    <strong>{entry.suiteName ?? "Ad-hoc run"}</strong>
                    <span className={`test-runner__status-badge test-runner__status-badge--${entry.status}`}>
                      {formatRunStatus(entry.status)}
                    </span>
                  </div>
                  <span>{entry.environmentName}</span>
                  <span>
                    {entry.passedCount} passed · {entry.failedCount} failed · {entry.skippedCount} skipped
                  </span>
                  <span>{formatTimestamp(entry.finishedAt ?? entry.startedAt)}</span>
                </button>
              ))}
            </div>
          ) : null}
        </section>

        <section className="test-runner__panel test-runner__panel--detail">
          <div className="test-runner__panel-header">
            <div>
              <span className="test-runner__eyebrow">Run detail</span>
              <h2>Per-case results, artifacts, and sanitized previews</h2>
            </div>
            <span>{runDetail ? `${runDetail.results.length} result rows` : "Select a run"}</span>
          </div>

          {!runDetail ? (
            <div className="test-runner__empty-panel">
              Empty state · Select a historical run to inspect per-case/per-row results, failure category, artifact links,
              and sanitized request/response previews.
            </div>
          ) : (
            <div className="test-runner__detail-stack">
              <article className="test-runner__summary-card">
                <div className="test-runner__status-row">
                  <div>
                    <h3>{runDetail.summary.suiteName ?? "Suite run"}</h3>
                    <p>{runDetail.summary.environmentName}</p>
                  </div>
                  <span className={`test-runner__status-badge test-runner__status-badge--${runDetail.summary.status}`}>
                    {formatRunStatus(runDetail.summary.status)}
                  </span>
                </div>
                <dl className="test-runner__metric-grid test-runner__metric-grid--four">
                  <div>
                    <dt>total</dt>
                    <dd>{runDetail.summary.totalCount}</dd>
                  </div>
                  <div>
                    <dt>passed</dt>
                    <dd>{runDetail.summary.passedCount}</dd>
                  </div>
                  <div>
                    <dt>failed</dt>
                    <dd>{runDetail.summary.failedCount}</dd>
                  </div>
                  <div>
                    <dt>skipped</dt>
                    <dd>{runDetail.summary.skippedCount}</dd>
                  </div>
                </dl>
                <p>
                  Started {formatTimestamp(runDetail.summary.startedAt)} · Finished {formatTimestamp(runDetail.summary.finishedAt)}
                </p>
              </article>

              <article className="test-runner__artifact-card">
                <div className="test-runner__subsection-header">
                  <div>
                    <span className="test-runner__eyebrow">Artifacts</span>
                    <h3>Artifact links</h3>
                  </div>
                  <span>{runDetail.artifacts.length} linked files</span>
                </div>
                {runDetail.artifacts.length === 0 ? <p className="test-runner__muted">No artifacts captured for this run.</p> : null}
                <div className="test-runner__artifact-list">
                  {runDetail.artifacts.map((artifact) => (
                    <a key={artifact.id} className="test-runner__artifact-link" href={artifact.filePath} target="_blank" rel="noreferrer">
                      <strong>{artifact.logicalName}</strong>
                      <span>{artifact.relativePath}</span>
                    </a>
                  ))}
                </div>
              </article>

              <div className="test-runner__result-list">
                {runDetail.results.map((result) => (
                  <article key={result.id} className={`test-runner__result-card test-runner__result-card--${result.status}`}>
                    <div className="test-runner__status-row">
                      <div>
                        <h3>{result.caseName}</h3>
                        <p>
                          {result.testCaseType.toUpperCase()} · {result.dataRowLabel ?? "Suite-level case"}
                        </p>
                      </div>
                      <span className={`test-runner__status-badge test-runner__status-badge--${result.status}`}>
                        {formatRunStatus(result.status)}
                      </span>
                    </div>

                    <dl className="test-runner__metric-grid test-runner__metric-grid--three">
                      <div>
                        <dt>failure category</dt>
                        <dd>{result.failureCategory}</dd>
                      </div>
                      <div>
                        <dt>duration</dt>
                        <dd>{result.durationMs} ms</dd>
                      </div>
                      <div>
                        <dt>error code</dt>
                        <dd>{result.errorCode ?? "none"}</dd>
                      </div>
                    </dl>

                    {result.errorMessage ? <p className="test-runner__error-copy">{result.errorMessage}</p> : null}

                    <div className="test-runner__preview-grid">
                      <div>
                        <h4>Sanitized request preview</h4>
                        <pre>{result.requestPreview}</pre>
                      </div>
                      <div>
                        <h4>Sanitized response preview</h4>
                        <pre>{result.responsePreview}</pre>
                      </div>
                    </div>

                    <div className="test-runner__preview-grid test-runner__preview-grid--single">
                      <div>
                        <h4>Assertion preview</h4>
                        <pre>{summarizeAssertions(result)}</pre>
                      </div>
                    </div>

                    <div className="test-runner__artifact-list">
                      {result.artifacts.length === 0 ? <span className="test-runner__muted">No per-row artifacts.</span> : null}
                      {result.artifacts.map((artifact) => (
                        <a key={artifact.id} className="test-runner__artifact-link" href={artifact.filePath} target="_blank" rel="noreferrer">
                          <strong>{artifact.logicalName}</strong>
                          <span>{artifact.relativePath}</span>
                        </a>
                      ))}
                    </div>
                  </article>
                ))}
              </div>
            </div>
          )}
        </section>
      </div>
    </section>
  );
}
