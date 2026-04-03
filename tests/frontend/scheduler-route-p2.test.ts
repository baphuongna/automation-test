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

function includesAll(source: string, fragments: string[]): boolean {
  return fragments.every((fragment) => source.includes(fragment));
}

const routeSource = readProjectFile("src/routes/test-runner.tsx");
const schedulerClientSource = readProjectFile("src/services/scheduler-client.ts");
const tsCommandSource = readProjectFile("src/types/commands.ts");
const tsDtoSource = readProjectFile("src/types/dto.ts");
const rustCommandContractSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustDtoContractSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const migrationPath = resolve("src-tauri/migrations/004_add_suite_schedules.sql");

assert(
  includesAll(routeSource, [
    'data-testid="route-test-runner"',
    "Schedule",
    "Enabled",
    "Disabled",
    "Last run",
    "Next run",
    "Diagnostics"
  ]),
  "P2-T7 phải embed scheduling UI copy trong src/routes/test-runner.tsx thay vì tạo route scheduling riêng."
);

assert(
  includesAll(routeSource, ["Runner control", "Run history", "Schedule", "Last run", "Next run"]),
  "P2-T7 scheduling surface phải coexist với runner/history hiện có trong test-runner thay vì tách execution model mới."
);

assert(
  includesAll(schedulerClientSource, [
    'invokeCommand("scheduler.schedule.list"',
    'invokeCommand("scheduler.schedule.upsert"',
    'invokeCommand("scheduler.schedule.setEnabled"',
    'invokeCommand("scheduler.schedule.delete"',
    "async listSchedules()",
    "async upsertSchedule(input:",
    "async setScheduleEnabled(input:",
    "async deleteSchedule(input:",
    'throw result.error ?? new Error("Missing command result for scheduler.schedule.list")'
  ]) &&
    !schedulerClientSource.includes("invoke("),
  "P2-T7 phải thêm thin typed scheduler client cho list/upsert/setEnabled/delete và vẫn giữ raw invoke phía sau tauri-client."
);

assert(
  includesAll(routeSource, [
    "schedulerClient.listSchedules()",
    "schedulerClient.upsertSchedule(",
    "schedulerClient.setScheduleEnabled(",
    "schedulerClient.deleteSchedule(",
    "setSchedules(",
    "scheduleForm",
    "cadenceMinutes",
    "Save schedule",
    "Update schedule",
    "Enable schedule",
    "Disable schedule",
    "Delete schedule",
    "Diagnostics",
    "No persisted schedules yet."
  ]),
  "P2-T7 route phải có scheduling UI thật với list/form/actions/diagnostics và wired qua scheduler-client."
);

assert(
  includesAll(tsDtoSource, [
    "export interface SuiteScheduleDto",
    "suiteId: EntityId",
    "environmentId: EntityId",
    "enabled: boolean",
    "cadenceMinutes: number",
    "lastRunAt?: IsoDateTime",
    "nextRunAt?: IsoDateTime",
    'lastRunStatus?: Exclude<RunStatus, "idle">',
    "lastError?: string",
    "createdAt: IsoDateTime",
    "updatedAt: IsoDateTime"
  ]),
  "P2-T7 phải thêm SuiteScheduleDto typed shape tối thiểu cho persisted schedule state."
);

assert(
  includesAll(tsCommandSource, [
    '"scheduler.schedule.list": Record<string, never>;',
    '"scheduler.schedule.upsert": {',
    "suiteId: EntityId",
    "environmentId: EntityId",
    "cadenceMinutes: number",
    "enabled: boolean",
    '"scheduler.schedule.setEnabled": {',
    '"scheduler.schedule.delete": {',
    '"scheduler.schedule.list": SuiteScheduleDto[];',
    '"scheduler.schedule.upsert": SuiteScheduleDto;',
    '"scheduler.schedule.setEnabled": SuiteScheduleDto;',
    '"scheduler.schedule.delete": { deleted: true };'
  ]),
  "P2-T7 phải thêm typed command contract tối thiểu cho schedule list/upsert/setEnabled/delete."
);

assert(
  includesAll(rustDtoContractSource, [
    "pub struct SuiteScheduleDto",
    "pub suite_id: EntityId",
    "pub environment_id: EntityId",
    "pub enabled: bool",
    "pub cadence_minutes: u32",
    "pub last_run_at: Option<IsoDateTime>",
    "pub next_run_at: Option<IsoDateTime>",
    "pub last_run_status: Option<RunStatus>",
    "pub last_error: Option<String>",
    "pub created_at: IsoDateTime",
    "pub updated_at: IsoDateTime"
  ]),
  "P2-T7 phải mirror SuiteScheduleDto sang Rust contract với persisted scheduling fields tương ứng."
);

assert(
  includesAll(rustCommandContractSource, [
    "pub struct SchedulerScheduleUpsertCommand",
    "pub suite_id: EntityId",
    "pub environment_id: EntityId",
    "pub cadence_minutes: u32",
    "pub enabled: bool",
    "pub struct SchedulerScheduleSetEnabledCommand",
    '#[serde(rename = "scheduler.schedule.list")]',
    '#[serde(rename = "scheduler.schedule.upsert")]',
    '#[serde(rename = "scheduler.schedule.setEnabled")]',
    '#[serde(rename = "scheduler.schedule.delete")]'
  ]),
  "P2-T7 phải mirror Rust command contracts cho scheduler schedule CRUD/read tối thiểu."
);

assert(
  existsSync(migrationPath),
  "P2-T7 phải tạo stub migration src-tauri/migrations/004_add_suite_schedules.sql để khóa persistence seam trong SQLite."
);

assert(
  includesAll(rustLibSource, [
    "fn scheduler_schedule_list(",
    "fn scheduler_schedule_upsert(",
    "fn scheduler_schedule_set_enabled(",
    "fn scheduler_schedule_delete(",
    "scheduler_schedule_list",
    "scheduler_schedule_upsert",
    "scheduler_schedule_set_enabled",
    "scheduler_schedule_delete"
  ]),
  "P2-T7 backend phải expose và register đầy đủ Tauri handlers cho scheduler.schedule.list/upsert/setEnabled/delete để tránh drift với scheduler-client.ts."
);

console.log("Scheduler route P2 regression/source test passed.");
