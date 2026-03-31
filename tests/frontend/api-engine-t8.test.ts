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

const dtoSource = readProjectFile("src/types/dto.ts");
const commandSource = readProjectFile("src/types/commands.ts");
const rustDtoContractsSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustCommandContractsSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const apiModelSource = readProjectFile("src-tauri/src/models/api_test_case.rs");
const apiRepositorySource = readProjectFile("src-tauri/src/repositories/api_repository.rs");
const apiServiceSource = readProjectFile("src-tauri/src/services/api_execution_service.rs");
const migration002Source = readProjectFile("src-tauri/migrations/002_add_api_endpoint_query_params.sql");

assert(
  dtoSource.includes("export interface ApiAuthDto") &&
    dtoSource.includes("auth?: ApiAuthDto") &&
    dtoSource.includes("export interface ApiExecutionResultDto") &&
    dtoSource.includes("export interface ApiAssertionResultDto"),
  "Shared DTO contracts must include API auth and assertion execution result structures for T8."
);

assert(
  commandSource.includes('"api.execute"') &&
    commandSource.includes("assertions: ApiAssertionDto[]") &&
    commandSource.includes("ApiExecutionResultDto"),
  "TypeScript command contract for api.execute must include assertions payload and structured execution response."
);

assert(
  rustDtoContractsSource.includes("pub struct ApiAuthDto") &&
    rustDtoContractsSource.includes("pub struct ApiExecutionResultDto") &&
    rustDtoContractsSource.includes("pub struct ApiAssertionResultDto"),
  "Rust DTO contracts must mirror API auth and assertion execution result structures for T8."
);

assert(
  rustCommandContractsSource.includes("pub assertions: Vec<ApiAssertionDto>") &&
    rustCommandContractsSource.includes("pub test_case_id: Option<EntityId>"),
  "Rust api.execute command must include assertions and optional testCaseId for persistence-aware execution."
);

assert(
  rustLibSource.includes("pub fn api_testcase_upsert") &&
    rustLibSource.includes("pub fn api_testcase_delete") &&
    rustLibSource.includes("pub async fn api_execute") &&
    rustLibSource.includes("Err(error @ TestForgeError::Validation(_)) => Ok(to_preflight_api_result(error))"),
  "Backend T8 handlers must expose api_testcase_upsert, api_testcase_delete, and api_execute."
);

assert(
  rustMainSource.includes("api_testcase_upsert") &&
    rustMainSource.includes("api_testcase_delete") &&
    rustMainSource.includes("api_execute"),
  "Tauri main invoke handler must register all T8 API commands."
);

assert(
  existsSync(resolve("src-tauri/src/models/api_test_case.rs")) &&
    existsSync(resolve("src-tauri/src/repositories/api_repository.rs")) &&
    existsSync(resolve("src-tauri/src/services/api_execution_service.rs")),
  "T8 must add dedicated API model/repository/service modules instead of overloading existing layers."
);

assert(
  apiServiceSource.includes("API_REQUEST_BUILD_FAILED: missing variable") &&
    apiServiceSource.includes("resolve_with_variables") &&
    apiServiceSource.includes("apply_supported_auth"),
  "API execution service must enforce missing-variable preflight failure before dispatch and supported auth application."
);

assert(
  apiServiceSource.includes("auth_preview") &&
    apiServiceSource.includes("[REDACTED]") &&
    apiServiceSource.includes("redact_sensitive_map"),
  "API execution previews must redact sensitive auth/header/query values."
);

assert(
  apiServiceSource.includes('failure_kind: Some("transport".to_string())') &&
    apiServiceSource.includes('Some("assertion".to_string())'),
  "API execution result must separate transport failures from assertion failures."
);

assert(
  apiModelSource.includes("pub query_params: BTreeMap<String, String>") &&
    apiModelSource.includes("query_params: BTreeMap::new()"),
  "ApiEndpoint model must persist query_params as first-class endpoint data."
);

assert(
  apiServiceSource.includes("endpoint.query_params = request.query_params.clone();"),
  "upsert_test_case must map request.queryParams into persistence model before repository write."
);

assert(
  apiRepositorySource.includes("query_params_json") &&
    apiRepositorySource.includes("serde_json::to_string(&endpoint.query_params)") &&
    apiRepositorySource.includes("query_params: serde_json::from_str(&query_params_json).unwrap_or_default()"),
  "ApiRepository upsert/find must round-trip query params via query_params_json storage."
);

assert(
  migration002Source.includes("ALTER TABLE api_endpoints") &&
    migration002Source.includes("query_params_json TEXT") &&
    migration002Source.includes("DEFAULT '{}'"),
  "T8 regression fix must add a forward migration for query_params_json persistence."
);

console.log("API engine T8 contract/regression test passed.");
