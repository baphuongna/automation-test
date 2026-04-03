import { useEffect, useMemo, useState } from "react";
import type { ReactElement } from "react";
import { environmentClient } from "../services/environment-client";
import { runnerClient } from "../services/runner-client";
import { useEnvStore } from "../store/env-store";
import { subscribeRunnerEvents, useRunStore } from "../store/run-store";
import type { RunCaseResultDto, RunDetailDto, RunHistoryEntryDto, RunHistoryGroupSummaryDto, SuiteDto } from "../types";

type ReportingRunStatusFilter = Exclude<RunHistoryEntryDto["status"], "idle">;

async function hydrateSelectedRun(options: {
  historyItems: RunHistoryEntryDto[];
  requestedRunId?: string | null;
  preserveSelectedRun?: boolean;
  previousSelectedRunId?: string | null;
}): Promise<{ selectedRunId: string | null; runDetail: RunDetailDto | null }> {
  const requestedRunId = options.requestedRunId ?? null;
  const previousSelectedRunId = options.previousSelectedRunId ?? null;
  const preserveSelectedRun = options.preserveSelectedRun ?? false;

  const nextSelectedRunId =
    requestedRunId && options.historyItems.some((entry) => entry.runId === requestedRunId)
      ? requestedRunId
      : preserveSelectedRun && previousSelectedRunId && options.historyItems.some((entry) => entry.runId === previousSelectedRunId)
        ? previousSelectedRunId
        : options.historyItems[0]?.runId ?? null;

  if (!nextSelectedRunId) {
    return {
      selectedRunId: null,
      runDetail: null
    };
  }

  const runDetail = await runnerClient.getRunDetail({ runId: nextSelectedRunId });
  return {
    selectedRunId: nextSelectedRunId,
    runDetail
  };
}

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

function normalizeDateInput(value: string): string | undefined {
  return value.trim().length > 0 ? value : undefined;
}

function toRfc3339Filter(value: string): string | undefined {
  const normalized = normalizeDateInput(value);
  if (!normalized) {
    return undefined;
  }

  const parsed = new Date(normalized);
  if (Number.isNaN(parsed.getTime())) {
    return undefined;
  }

  return parsed.toISOString();
}

function resolveReportingDateFilters(startedAfter: string, startedBefore: string): {
  startedAfter?: string;
  startedBefore?: string;
} {
  const normalizedStartedAfter = normalizeDateInput(startedAfter);
  const normalizedStartedBefore = normalizeDateInput(startedBefore);

  const startedAfterFilter = toRfc3339Filter(startedAfter);
  if (normalizedStartedAfter && !startedAfterFilter) {
    throw new Error("Started after filter must be a valid date/time.");
  }

  const startedBeforeFilter = toRfc3339Filter(startedBefore);
  if (normalizedStartedBefore && !startedBeforeFilter) {
    throw new Error("Started before filter must be a valid date/time.");
  }

  return {
    ...(startedAfterFilter ? { startedAfter: startedAfterFilter } : {}),
    ...(startedBeforeFilter ? { startedBefore: startedBeforeFilter } : {})
  };
}

function formatPassRate(passedRuns: number, totalRuns: number): string {
  if (totalRuns <= 0) {
    return "0%";
  }

  return `${Math.round((passedRuns / totalRuns) * 100)}%`;
}

function formatDurationAverage(historyItems: RunHistoryEntryDto[]): string {
  const durations = historyItems
    .map((entry) => {
      const startedAt = new Date(entry.startedAt).getTime();
      const finishedAt = entry.finishedAt ? new Date(entry.finishedAt).getTime() : Number.NaN;
      if (Number.isNaN(startedAt) || Number.isNaN(finishedAt) || finishedAt < startedAt) {
        return null;
      }

      return finishedAt - startedAt;
    })
    .filter((value): value is number => value !== null);

  if (durations.length === 0) {
    return "No completed durations yet.";
  }

  const averageMs = Math.round(durations.reduce((sum, value) => sum + value, 0) / durations.length);
  return `${averageMs} ms`;
}

function isCompletedRun(entry: RunHistoryEntryDto): boolean {
  return Boolean(entry.finishedAt) && (entry.status === "passed" || entry.status === "failed" || entry.status === "cancelled");
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
  const [reportSuiteFilterId, setReportSuiteFilterId] = useState<string | null>(null);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetailDto | null>(null);
  const [historyGroupSummary, setHistoryGroupSummary] = useState<RunHistoryGroupSummaryDto>({
    totalRuns: 0,
    passedRuns: 0,
    failedRuns: 0,
    cancelledRuns: 0,
    failureCategoryCounts: []
  });
  const [reportStatusFilter, setReportStatusFilter] = useState<ReportingRunStatusFilter | "">("");
  const [reportStartedAfter, setReportStartedAfter] = useState("");
  const [reportStartedBefore, setReportStartedBefore] = useState("");
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
  const isStopping = useRunStore((state) => state.isStopping);
  const terminalMessage = useRunStore((state) => state.terminalMessage);

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

  const groupedFailedResults = useMemo(() => {
    const failureGroups = new Map<string, RunCaseResultDto[]>();
    (runDetail?.results ?? [])
      .filter((result) => result.status === "failed")
      .forEach((result) => {
        const group = failureGroups.get(result.failureCategory) ?? [];
        group.push(result);
        failureGroups.set(result.failureCategory, group);
      });

    return Array.from(failureGroups.entries()).map(([failureCategory, results]) => ({
      failureCategory,
      results
    }));
  }, [runDetail]);

  const trendReadyAggregates = useMemo(() => {
    const completedRuns = history.filter(isCompletedRun);
    const latestCompletedRun = completedRuns[0] ?? null;
    const completedRunCount = historyGroupSummary.passedRuns + historyGroupSummary.failedRuns + historyGroupSummary.cancelledRuns;
    return {
      latestRunFinishedAt: latestCompletedRun?.finishedAt
        ? formatTimestamp(latestCompletedRun.finishedAt)
        : "No completed runs in the selected reporting window.",
      passRate: formatPassRate(historyGroupSummary.passedRuns, completedRunCount),
      averageDuration: formatDurationAverage(history)
    };
  }, [historyGroupSummary.cancelledRuns, historyGroupSummary.failedRuns, historyGroupSummary.passedRuns, history]);

  async function loadRunnerScreen(options: { preserveSelectedRun?: boolean; requestedRunId?: string | null } = {}): Promise<void> {
    const preserveSelectedRun = options.preserveSelectedRun ?? false;

    setErrorMessage(null);

    const historyFilters: Parameters<typeof runnerClient.listRunHistory>[0] = {};
    if (reportSuiteFilterId) {
      historyFilters.suiteId = reportSuiteFilterId;
    }
    if (reportStatusFilter) {
      historyFilters.status = reportStatusFilter;
    }
    const dateFilters = resolveReportingDateFilters(reportStartedAfter, reportStartedBefore);
    if (dateFilters.startedAfter) {
      historyFilters.startedAfter = dateFilters.startedAfter;
    }
    if (dateFilters.startedBefore) {
      historyFilters.startedBefore = dateFilters.startedBefore;
    }

    const [suiteItems, environmentItems, historyReport] = await Promise.all([
      runnerClient.listSuites(),
      environmentClient.list(),
      runnerClient.listRunHistoryReport(historyFilters)
    ]);
    const historyItems = historyReport.entries;

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
    setHistoryGroupSummary(historyReport.groupSummary);

    const selection = await hydrateSelectedRun({
      historyItems,
      requestedRunId: options.requestedRunId ?? null,
      preserveSelectedRun,
      previousSelectedRunId: selectedRunId
    });

    setSelectedRunId(selection.selectedRunId);
    setRunDetail(selection.runDetail);
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
  }, [reportStartedAfter, reportStartedBefore, reportStatusFilter, reportSuiteFilterId]);

  function handleResetFilters(): void {
    setReportSuiteFilterId(null);
    setReportStatusFilter("");
    setReportStartedAfter("");
    setReportStartedBefore("");
    setFeedbackMessage("Reporting filters reset. Showing the latest operational window again.");
  }

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
      await loadRunnerScreen({ preserveSelectedRun: true, requestedRunId: nextRunId ?? null });
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
      setSelectedRunId(result.runId);
      setRunDetail(null);
      setFeedbackMessage(`Runner control accepted · ${result.suite.name} started as ${result.runId}.`);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể chạy suite.");
      setIsExecuting(false);
    }
  }

  async function handleCancelSuite(): Promise<void> {
    if (!activeRunId) {
      setFeedbackMessage("No active run right now.");
      return;
    }

    if (isCancelling || isStopping) {
      setFeedbackMessage("Already cancelling the active run. Waiting for terminal update.");
      return;
    }

    try {
      setErrorMessage(null);
      setIsCancelling(true);
      useRunStore.getState().setRunState({
        activeRunId,
        isStopping: true,
        terminalMessage: "Cancel requested. Waiting for terminal runner update.",
        progress,
        status: runStatus
      });
      const result = await runnerClient.cancelSuite({ runId: activeRunId });
      if (result.cancelled) {
        setFeedbackMessage(`Cancel requested for active run ${activeRunId}.`);
      } else {
        setFeedbackMessage("No active run right now.");
        setIsCancelling(false);
        useRunStore.getState().setRunState({
          activeRunId: null,
          isStopping: false,
          terminalMessage: "No active run right now.",
          progress,
          status: "idle"
        });
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể cancel suite run.");
      setIsCancelling(false);
      useRunStore.getState().setRunState({
        activeRunId,
        isStopping: false,
        terminalMessage: null,
        progress,
        status: runStatus
      });
    }
  }

  async function handleRerunFailed(): Promise<void> {
    if (!selectedRun || !selectedRun.suiteId || !activeEnvironmentId) {
      setErrorMessage("Select a persisted suite run and environment before rerunning failures.");
      return;
    }

    if (activeRunId || isExecuting || isRerunningFailed) {
      setFeedbackMessage("Wait for the active run to finish before rerunning failed targets.");
      return;
    }

    try {
      setErrorMessage(null);
      setIsRerunningFailed(true);
      const result = await runnerClient.executeSuite({
        suiteId: selectedRun.suiteId,
        environmentId: activeEnvironmentId,
        rerunFailedFromRunId: selectedRun.runId
      });
      setSelectedRunId(result.runId);
      setRunDetail(null);
      setFeedbackMessage(`Rerun failed accepted from historical run ${selectedRun.runId}.`);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Không thể rerun failed.");
      setIsRerunningFailed(false);
    }
  }

  const liveCompleted = progress?.completed ?? 0;
  const liveTotal = progress?.total ?? selectedRun?.totalCount ?? 0;
  const livePercent = buildProgressPercent(liveCompleted, liveTotal);
  const canRun = Boolean(selectedSuite && activeEnvironmentId) && !isExecuting && !activeRunId;
  const canRerunFailed =
    Boolean(selectedRun?.suiteId && selectedRun.failedCount > 0 && activeEnvironmentId) &&
    !isRerunningFailed &&
    !isExecuting &&
    !activeRunId;

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
            disabled={!activeRunId || isCancelling || isStopping}
          >
            {isCancelling ? "Cancelling…" : "Cancel active run"}
          </button>
        </div>
      </header>

      {feedbackMessage ? <div className="test-runner__feedback">{feedbackMessage}</div> : null}
      {!feedbackMessage && terminalMessage ? <div className="test-runner__feedback">{terminalMessage}</div> : null}
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
                {isStopping ? <p>Cancel requested. Waiting for terminal runner update.</p> : null}
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

          <article className="test-runner__summary-card">
            <div className="test-runner__subsection-header">
              <div>
                <span className="test-runner__eyebrow">Reporting filters</span>
                <h3>Operational reporting window</h3>
              </div>
              <button type="button" className="test-runner__ghost-action" onClick={handleResetFilters}>
                Reset filters
              </button>
            </div>

            <div className="test-runner__preview-grid">
              <label className="test-runner__field">
                <span>Suite scope</span>
                <select value={reportSuiteFilterId ?? ""} onChange={(event) => setReportSuiteFilterId(event.target.value || null)}>
                  <option value="">All suites</option>
                  {suites.map((suite) => (
                    <option key={suite.id} value={suite.id}>
                      {suite.name}
                    </option>
                  ))}
                </select>
              </label>

              <label className="test-runner__field">
                <span>Run status</span>
                <select value={reportStatusFilter} onChange={(event) => setReportStatusFilter(event.target.value as ReportingRunStatusFilter | "") }>
                  <option value="">All statuses</option>
                  <option value="passed">Passed</option>
                  <option value="failed">Failed</option>
                  <option value="cancelled">Cancelled</option>
                  <option value="running">Running</option>
                  <option value="queued">Queued</option>
                  <option value="skipped">Skipped</option>
                </select>
              </label>
            </div>

            <div className="test-runner__preview-grid">
              <label className="test-runner__field">
                <span>Started after</span>
                <input
                  type="datetime-local"
                  value={reportStartedAfter}
                  onChange={(event) => setReportStartedAfter(normalizeDateInput(event.target.value) ?? "")}
                />
              </label>

              <label className="test-runner__field">
                <span>Started before</span>
                <input
                  type="datetime-local"
                  value={reportStartedBefore}
                  onChange={(event) => setReportStartedBefore(normalizeDateInput(event.target.value) ?? "")}
                />
              </label>
            </div>
          </article>

          <article className="test-runner__summary-card">
            <div className="test-runner__subsection-header">
              <div>
                <span className="test-runner__eyebrow">Filtered run summary</span>
                <h3>Grouped summary for the active reporting window</h3>
              </div>
              <span>{historyGroupSummary.totalRuns} total runs</span>
            </div>
            <dl className="test-runner__metric-grid test-runner__metric-grid--four">
              <div>
                <dt>total runs</dt>
                <dd>{historyGroupSummary.totalRuns}</dd>
              </div>
              <div>
                <dt>passed runs</dt>
                <dd>{historyGroupSummary.passedRuns}</dd>
              </div>
              <div>
                <dt>failed runs</dt>
                <dd>{historyGroupSummary.failedRuns}</dd>
              </div>
              <div>
                <dt>cancelled runs</dt>
                <dd>{historyGroupSummary.cancelledRuns}</dd>
              </div>
            </dl>
            <div className="test-runner__subsection-header">
              <strong>Failure groups</strong>
              <span>{historyGroupSummary.failureCategoryCounts.length} categories</span>
            </div>
            {historyGroupSummary.failureCategoryCounts.length === 0 ? (
              <p className="test-runner__muted">No failed results match the active reporting filters.</p>
            ) : (
              <div className="test-runner__artifact-list">
                {historyGroupSummary.failureCategoryCounts.map((group) => (
                  <span key={group.category} className="test-runner__artifact-link">
                    <strong>{group.category}</strong>
                    <span>{group.count} failed result(s)</span>
                  </span>
                ))}
              </div>
            )}
          </article>

          <article className="test-runner__summary-card">
            <div className="test-runner__subsection-header">
              <div>
                <span className="test-runner__eyebrow">Trend-ready aggregates</span>
                <h3>Lightweight aggregate presentation</h3>
              </div>
              <span>Operational view only</span>
            </div>
            {history.length === 0 ? <p className="test-runner__muted">No persisted runs in the selected reporting window.</p> : null}
            <dl className="test-runner__metric-grid test-runner__metric-grid--three">
              <div>
                <dt>Latest run finished</dt>
                <dd>{trendReadyAggregates.latestRunFinishedAt}</dd>
              </div>
              <div>
                <dt>Pass rate</dt>
                <dd>{trendReadyAggregates.passRate}</dd>
              </div>
              <div>
                <dt>Average duration</dt>
                <dd>{trendReadyAggregates.averageDuration}</dd>
              </div>
            </dl>
          </article>

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

              <article className="test-runner__artifact-card">
                <div className="test-runner__subsection-header">
                  <div>
                    <span className="test-runner__eyebrow">Failed-case drilldown</span>
                    <h3>Group failed results by failure category</h3>
                  </div>
                  <span>{groupedFailedResults.length} groups</span>
                </div>
                {groupedFailedResults.length === 0 ? <p className="test-runner__muted">No failed results match the active reporting filters.</p> : null}
                <div className="test-runner__detail-stack">
                  {groupedFailedResults.map((group) => (
                    <article key={group.failureCategory} className="test-runner__result-card test-runner__result-card--failed">
                      <div className="test-runner__status-row">
                        <strong>{group.failureCategory}</strong>
                        <span>{group.results.length} failed result(s)</span>
                      </div>
                      <div className="test-runner__artifact-list">
                        {group.results.map((result) => {
                          const firstArtifact = result.artifacts[0] ?? null;
                          return firstArtifact ? (
                            <a key={result.id} className="test-runner__artifact-link" href={firstArtifact.filePath} target="_blank" rel="noreferrer">
                              <strong>{result.caseName}</strong>
                              <span>{firstArtifact.relativePath}</span>
                            </a>
                          ) : (
                            <span key={result.id} className="test-runner__muted">
                              {result.caseName} · Missing artifact for this failed result.
                            </span>
                          );
                        })}
                      </div>
                    </article>
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
