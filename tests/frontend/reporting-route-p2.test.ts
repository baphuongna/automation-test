import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(message);
  }
}

function readProjectFile(relativePath: string): string {
  const absolutePath = resolve(relativePath);
  assert(existsSync(absolutePath), `Expected file to exist: ${relativePath}`);
  return readFileSync(absolutePath, "utf8");
}

const clientSource = readProjectFile("src/services/runner-client.ts");
const commandSource = readProjectFile("src/types/commands.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const routeSource = readProjectFile("src/routes/test-runner.tsx");

assert(
  clientSource.includes("runner.run.history") &&
    clientSource.includes('type RunHistoryStatusFilter = Exclude<RunHistoryEntryDto["status"], "idle">;') &&
    clientSource.includes("status?: RunHistoryStatusFilter") &&
    clientSource.includes("startedAfter?: string") &&
    clientSource.includes("startedBefore?: string") &&
    clientSource.includes("const payload = {") &&
    clientSource.includes("...(input.status ? { status: input.status } : {})"),
  "P2-T6 reporting client seam phải nhận suite/status/date filters trên runner.run.history."
);

assert(
  commandSource.includes('"runner.run.history": {') &&
    commandSource.includes('status?: Exclude<RunStatus, "idle">') &&
    commandSource.includes("startedAfter?: IsoDateTime") &&
    commandSource.includes("startedBefore?: IsoDateTime"),
  "P2-T6 typed command map phải giữ reporting filters dưới runner.run.history thay vì mở route/command analytics riêng."
);

assert(
  dtoSource.includes("export interface RunHistoryFilterDto") &&
    dtoSource.includes('status?: Exclude<RunStatus, "idle">') &&
    dtoSource.includes("startedAfter?: IsoDateTime") &&
    dtoSource.includes("startedBefore?: IsoDateTime") &&
    dtoSource.includes("export interface RunHistoryGroupSummaryDto") &&
    dtoSource.includes("failureCategoryCounts") &&
    !dtoSource.includes("plaintextSecret") &&
    !dtoSource.includes("ciphertextSecret") &&
    !dtoSource.includes("maskedSecretValue"),
  "P2-T6 DTOs chỉ được thêm minimal reporting filter/group summary shapes và không được lộ secret fields mới."
);

assert(
  routeSource.includes("const [reportStatusFilter, setReportStatusFilter] = useState") &&
    routeSource.includes("const [reportStartedAfter, setReportStartedAfter] = useState") &&
    routeSource.includes("const [reportStartedBefore, setReportStartedBefore] = useState") &&
    routeSource.includes("Reporting filters") &&
    routeSource.includes("Suite scope") &&
    routeSource.includes("Run status") &&
    routeSource.includes("Started after") &&
    routeSource.includes("Started before") &&
    routeSource.includes("Reset filters"),
  "P2-T6 route phải thêm local reporting filter state và filter controls ngay trong test-runner reporting surface."
);

assert(
  routeSource.includes("const [historyGroupSummary, setHistoryGroupSummary] = useState<RunHistoryGroupSummaryDto>(") &&
    routeSource.includes("const groupedFailedResults = useMemo(") &&
    routeSource.includes("const trendReadyAggregates = useMemo(") &&
    routeSource.includes("setHistoryGroupSummary(historyReport.groupSummary)") &&
    routeSource.includes("Filtered run summary") &&
    routeSource.includes("Failure groups") &&
    routeSource.includes("Trend-ready aggregates") &&
    routeSource.includes("Operational view only"),
  "P2-T6 route phải derive grouped summary, failed grouping và trend-ready aggregates ngay từ read model hiện có."
);

assert(
  routeSource.includes("Failed-case drilldown") &&
    routeSource.includes("Group failed results by failure category") &&
    routeSource.includes("Missing artifact for this failed result.") &&
    routeSource.includes("No failed results match the active reporting filters.") &&
    routeSource.includes("No persisted runs in the selected reporting window."),
  "P2-T6 route phải có grouped failed-case drilldown cùng empty state và missing-artifact state rõ ràng, vẫn giữ surface reporting nhẹ và vận hành được."
);

console.log("Reporting route P2 regression/source test passed.");
