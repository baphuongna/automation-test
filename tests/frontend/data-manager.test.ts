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

const commandsSource = readProjectFile("src/types/commands.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const dataManagerRouteSource = readProjectFile("src/routes/data-manager.tsx");
const dataClientSource = readProjectFile("src/services/data-table-client.ts");
const previewClientSource = readProjectFile("src/services/data-table-preview-client.ts");
const tauriClientSource = readProjectFile("src/services/tauri-client.ts");
const rustCommandContractsSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustDtoContractsSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");

assert(
  dtoSource.includes("export interface DataTableDto") &&
    dtoSource.includes("export interface DataTableRowDto") &&
    dtoSource.includes("export interface DataTableAssociationMetadataDto"),
  "Shared DTO contracts must expose data tables, rows, and association-ready metadata for T7."
);

assert(
  commandsSource.includes('"dataTable.list"') &&
    commandsSource.includes('"dataTable.create"') &&
    commandsSource.includes('"dataTable.update"') &&
    commandsSource.includes('"dataTable.delete"') &&
    commandsSource.includes('"dataTable.row.upsert"') &&
    commandsSource.includes('"dataTable.row.delete"') &&
    commandsSource.includes('"dataTable.import"') &&
    commandsSource.includes('"dataTable.export"'),
  "Command contracts must include the full T7 data-table CRUD and import/export surface."
);

assert(
  dataClientSource.includes('"__TAURI_INTERNALS__" in window') &&
    dataClientSource.includes("dataTablePreviewClient") &&
    dataClientSource.includes("Preview fallback active"),
  "Data table client must mirror T6 preview-fallback detection and stay bounded behind one typed client."
);

assert(
  previewClientSource.includes("localStorage") &&
    previewClientSource.includes("zeroEnabled") &&
    previewClientSource.includes("Malformed") &&
    previewClientSource.includes("associationMeta"),
  "Preview adapter must persist browser QA state and cover zero-enabled, malformed import, and association metadata paths."
);

assert(
  dataManagerRouteSource.includes("empty table") &&
    dataManagerRouteSource.includes("zero enabled rows") &&
    dataManagerRouteSource.includes("Import failed") &&
    dataManagerRouteSource.includes("window.confirm") &&
    dataManagerRouteSource.includes("association metadata"),
  "Data Manager route must render T7 empty/zero-enabled/import-error handling and confirmation flows."
);

assert(
  !dataManagerRouteSource.includes("invoke(") && !dataClientSource.includes("invoke("),
  "T7 frontend code must not call raw invoke() outside the shared tauri client boundary."
);

assert(
  tauriClientSource.includes("command.replaceAll") && tauriClientSource.includes("{ payload }"),
  "Shared Tauri client must remain the only command-name translation and payload wrapper boundary used by T7."
);

assert(
  rustDtoContractsSource.includes("pub struct DataTableDto") &&
    rustDtoContractsSource.includes("pub struct DataTableRowDto") &&
    rustDtoContractsSource.includes("pub struct DataTableAssociationMetadataDto"),
  "Rust DTO contracts must mirror the T7 data table payloads."
);

assert(
  rustCommandContractsSource.includes("DataTableCreateCommand") &&
    rustCommandContractsSource.includes("DataTableUpdateCommand") &&
    rustCommandContractsSource.includes("DataTableImportCommand") &&
    rustCommandContractsSource.includes("dataTable.import") &&
    rustCommandContractsSource.includes("dataTable.export"),
  "Rust command contracts must mirror T7 CRUD and import/export commands."
);

assert(
  rustLibSource.includes("fn data_table_list") &&
    rustLibSource.includes("fn data_table_create") &&
    rustLibSource.includes("fn data_table_import") &&
    rustLibSource.includes("fn data_table_export"),
  "Rust Tauri handlers must expose the T7 data table commands."
);

assert(
  rustLibSource.includes("tauri::generate_handler![") &&
    rustLibSource.includes("data_table_list") &&
    rustLibSource.includes("data_table_create") &&
    rustLibSource.includes("data_table_import") &&
    rustLibSource.includes("data_table_export") &&
    rustMainSource.includes("testforge::run();"),
  "Tauri bootstrap must register the T7 data table handlers through the library run() entrypoint."
);

console.log("Data Manager T7 contract and UI regression test passed.");
