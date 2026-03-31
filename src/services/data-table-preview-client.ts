import type {
  DataTableAssociationMetadataDto,
  DataTableColumnDto,
  DataTableDto,
  DataTableExportDto,
  DataTableImportResultDto,
  DataTableRowDto
} from "../types";

const PREVIEW_STORAGE_KEY = "testforge.dataManager.preview.v1";
const PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T7 verification path.";

interface PreviewState {
  tables: DataTableDto[];
}

interface DataTableSaveInput {
  id?: string;
  name: string;
  description?: string;
  columns: DataTableColumnDto[];
}

interface DataTableRowSaveInput {
  tableId: string;
  row: {
    id?: string;
    values: string[];
    enabled: boolean;
    rowIndex: number;
  };
}

interface DataTableImportInput {
  tableId?: string;
  name: string;
  description?: string;
  format: "csv" | "json";
  content: string;
}

function nowIso(): string {
  return new Date().toISOString();
}

function createId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

function canUseBrowserStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function createAssociationMeta(rows: DataTableRowDto[]): DataTableAssociationMetadataDto {
  return {
    canAssociateToTestCases: true,
    linkedTestCaseIds: [],
    totalRowCount: rows.length,
    enabledRowCount: rows.filter((row) => row.enabled).length
  };
}

function withAssociationMeta(table: Omit<DataTableDto, "associationMeta">): DataTableDto {
  return {
    ...table,
    associationMeta: createAssociationMeta(table.rows)
  };
}

function createSeedState(): PreviewState {
  const timestamp = nowIso();

  return {
    tables: [
      withAssociationMeta({
        id: "table-preview-users",
        name: "Users Preview",
        description: "Preview table for CRUD and import/export QA.",
        columns: [
          { name: "username", colType: "string" },
          { name: "password", colType: "string" }
        ],
        rows: [
          {
            id: "row-preview-alice",
            values: ["alice", "secret-1"],
            enabled: true,
            rowIndex: 0,
            createdAt: timestamp,
            updatedAt: timestamp
          },
          {
            id: "row-preview-bob",
            values: ["bob", "secret-2"],
            enabled: false,
            rowIndex: 1,
            createdAt: timestamp,
            updatedAt: timestamp
          }
        ],
        createdAt: timestamp,
        updatedAt: timestamp
      }),
      withAssociationMeta({
        id: "table-preview-zeroEnabled",
        name: "zeroEnabled Preview",
        description: "All rows are disabled so zero enabled rows can be exercised.",
        columns: [{ name: "email", colType: "string" }],
        rows: [
          {
            id: "row-preview-disabled",
            values: ["nobody@example.com"],
            enabled: false,
            rowIndex: 0,
            createdAt: timestamp,
            updatedAt: timestamp
          }
        ],
        createdAt: timestamp,
        updatedAt: timestamp
      })
    ]
  };
}

function readStoredState(): PreviewState {
  if (!canUseBrowserStorage()) {
    return createSeedState();
  }

  const raw = window.localStorage.getItem(PREVIEW_STORAGE_KEY);
  if (!raw) {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }

  try {
    return JSON.parse(raw) as PreviewState;
  } catch {
    const seed = createSeedState();
    window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(seed));
    return seed;
  }
}

function writeStoredState(state: PreviewState): void {
  if (!canUseBrowserStorage()) {
    return;
  }

  window.localStorage.setItem(PREVIEW_STORAGE_KEY, JSON.stringify(state));
}

function normalizeColumns(columns: DataTableColumnDto[]): DataTableColumnDto[] {
  return columns
    .map((column) => ({
      name: column.name.trim(),
      colType: column.colType.trim() || "string"
    }))
    .filter((column) => column.name.length > 0);
}

function ensureTableInput(input: DataTableSaveInput): DataTableColumnDto[] {
  const trimmedName = input.name.trim();
  const normalizedColumns = normalizeColumns(input.columns);

  if (trimmedName.length === 0) {
    throw new Error("Table name is required.");
  }

  if (normalizedColumns.length === 0) {
    throw new Error("At least one column is required.");
  }

  return normalizedColumns;
}

function ensureRowMatchesColumns(rowValues: string[], columnCount: number): void {
  if (rowValues.length !== columnCount) {
    throw new Error("Row value count must match the number of columns.");
  }
}

function normalizeDescription(description: string | undefined): string | undefined {
  const trimmed = description?.trim();
  return trimmed ? trimmed : undefined;
}

function createTableDraft(
  table: Omit<DataTableDto, "associationMeta" | "description"> & { description?: string }
): Omit<DataTableDto, "associationMeta"> {
  const description = normalizeDescription(table.description);
  return description === undefined ? table : { ...table, description };
}

function withOptionalDescription<T extends object>(payload: T, description: string | undefined): T & { description?: string } {
  const normalized = normalizeDescription(description);
  return normalized === undefined ? payload : { ...payload, description: normalized };
}

function parseCsvImport(content: string): { columns: DataTableColumnDto[]; rows: Array<{ values: string[]; enabled: boolean }> } {
  const trimmedContent = content.trim();
  if (trimmedContent.length === 0) {
    throw new Error("Malformed CSV import: content is empty.");
  }

  const lines = trimmedContent
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length < 1) {
    throw new Error("Malformed CSV import: header row is required.");
  }

  const headerLine = lines[0];
  if (!headerLine) {
    throw new Error("Malformed CSV import: header row is required.");
  }

  const header = headerLine.split(",").map((item) => item.trim());
  if (header.length === 0 || header.some((item) => item.length === 0)) {
    throw new Error("Malformed CSV import: header row contains empty columns.");
  }

  const columns = header.map((name) => ({ name, colType: "string" }));
  const rows = lines.slice(1).map((line, index) => {
    const values = line.split(",").map((item) => item.trim());
    if (values.length !== header.length) {
      throw new Error(`Malformed CSV import: row ${index + 2} has ${values.length} values, expected ${header.length}.`);
    }

    return { values, enabled: true };
  });

  return { columns, rows };
}

function parseJsonImport(content: string): { columns: DataTableColumnDto[]; rows: Array<{ values: string[]; enabled: boolean }> } {
  const trimmedContent = content.trim();
  if (trimmedContent.length === 0) {
    throw new Error("Malformed JSON import: content is empty.");
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmedContent);
  } catch {
    throw new Error("Malformed JSON import: content is not valid JSON.");
  }

  if (typeof parsed !== "object" || parsed === null) {
    throw new Error("Malformed JSON import: expected a table object.");
  }

  const record = parsed as {
    columns?: Array<{ name?: string; colType?: string } | string>;
    rows?: Array<{ values?: string[]; enabled?: boolean }>;
  };

  if (!Array.isArray(record.columns) || !Array.isArray(record.rows)) {
    throw new Error("Malformed JSON import: expected columns and rows arrays.");
  }

  const columns = record.columns
    .map((column) =>
      typeof column === "string"
        ? { name: column.trim(), colType: "string" }
        : { name: column.name?.trim() ?? "", colType: column.colType?.trim() || "string" }
    )
    .filter((column) => column.name.length > 0);

  if (columns.length === 0) {
    throw new Error("Malformed JSON import: at least one column is required.");
  }

  const rows = record.rows.map((row, index) => {
    if (!Array.isArray(row.values)) {
      throw new Error(`Malformed JSON import: row ${index + 1} is missing a values array.`);
    }

    ensureRowMatchesColumns(row.values, columns.length);
    return {
      values: row.values.map((value) => `${value ?? ""}`),
      enabled: row.enabled ?? true
    };
  });

  return { columns, rows };
}

function parseImport(input: DataTableImportInput) {
  return input.format === "csv" ? parseCsvImport(input.content) : parseJsonImport(input.content);
}

function toCsv(table: DataTableDto): string {
  const header = table.columns.map((column) => column.name).join(",");
  const rows = table.rows.map((row) => row.values.join(","));
  return [header, ...rows].join("\n");
}

function toJson(table: DataTableDto): string {
  return JSON.stringify(
    {
      name: table.name,
      description: table.description,
      columns: table.columns,
      rows: table.rows.map((row) => ({
        values: row.values,
        enabled: row.enabled,
        rowIndex: row.rowIndex
      })),
      associationMeta: table.associationMeta
    },
    null,
    2
  );
}

export const dataTablePreviewClient = {
  banner: PREVIEW_FALLBACK_BANNER,

  isAvailable(): boolean {
    return typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);
  },

  list(): Promise<DataTableDto[]> {
    return Promise.resolve(readStoredState().tables);
  },

  create(input: DataTableSaveInput): Promise<DataTableDto> {
    const columns = ensureTableInput(input);
    const state = readStoredState();
    const timestamp = nowIso();
      const table = withAssociationMeta(
        createTableDraft(
          withOptionalDescription(
            {
              id: createId("table-preview"),
              name: input.name.trim(),
              columns,
              rows: [],
              createdAt: timestamp,
              updatedAt: timestamp
            },
            input.description
          )
        )
      );

    writeStoredState({ tables: [...state.tables, table] });
    return Promise.resolve(table);
  },

  update(input: DataTableSaveInput & { id: string }): Promise<DataTableDto> {
    const columns = ensureTableInput(input);
    const state = readStoredState();
    const timestamp = nowIso();
    let updated: DataTableDto | null = null;

    const tables = state.tables.map((table) => {
      if (table.id !== input.id) {
        return table;
      }

      const nextRows = table.rows.map((row) => {
        ensureRowMatchesColumns(row.values, columns.length);
        return row;
      });

      const nextTable = withAssociationMeta(
        createTableDraft(
          withOptionalDescription(
            {
              ...table,
              name: input.name.trim(),
              columns,
              rows: nextRows,
              updatedAt: timestamp
            },
            input.description
          )
        )
      );

      updated = nextTable;

      return nextTable;
    });

    if (!updated) {
      throw new Error("Không tìm thấy bảng preview để cập nhật.");
    }

    writeStoredState({ tables });
    return Promise.resolve(updated);
  },

  remove(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    writeStoredState({ tables: state.tables.filter((table) => table.id !== id) });
    return Promise.resolve({ deleted: true });
  },

  upsertRow(input: DataTableRowSaveInput): Promise<DataTableRowDto> {
    const state = readStoredState();
    const timestamp = nowIso();
    let savedRow: DataTableRowDto | null = null;

    const tables = state.tables.map((table) => {
      if (table.id !== input.tableId) {
        return table;
      }

      ensureRowMatchesColumns(input.row.values, table.columns.length);

      const nextRow: DataTableRowDto = {
        id: input.row.id ?? createId("row-preview"),
        values: input.row.values.map((value) => value.trim()),
        enabled: input.row.enabled,
        rowIndex: input.row.rowIndex,
        createdAt:
          table.rows.find((row) => row.id === input.row.id)?.createdAt ?? timestamp,
        updatedAt: timestamp
      };

      const existingRows = table.rows.filter((row) => row.id !== nextRow.id);
      const rows = [...existingRows, nextRow].sort((left, right) => left.rowIndex - right.rowIndex);
      savedRow = nextRow;

      return withAssociationMeta({
        ...table,
        rows,
        updatedAt: timestamp
      });
    });

    if (!savedRow) {
      throw new Error("Không tìm thấy bảng preview để lưu dòng dữ liệu.");
    }

    writeStoredState({ tables });
    return Promise.resolve(savedRow);
  },

  deleteRow(id: string): Promise<{ deleted: true }> {
    const state = readStoredState();
    const timestamp = nowIso();
    const tables = state.tables.map((table) =>
      withAssociationMeta({
        ...table,
        rows: table.rows.filter((row) => row.id !== id),
        updatedAt: timestamp
      })
    );

    writeStoredState({ tables });
    return Promise.resolve({ deleted: true });
  },

  importTable(input: DataTableImportInput): Promise<DataTableImportResultDto> {
    const parsed = parseImport(input);
    const state = readStoredState();
    const timestamp = nowIso();
    const rows: DataTableRowDto[] = parsed.rows.map((row, index) => ({
      id: createId("row-preview-import"),
      values: row.values,
      enabled: row.enabled,
      rowIndex: index,
      createdAt: timestamp,
      updatedAt: timestamp
    }));

    const nextTable = withAssociationMeta(
      createTableDraft(
        withOptionalDescription(
          {
            id: input.tableId ?? createId("table-preview-import"),
            name: input.name.trim(),
            columns: parsed.columns,
            rows,
            createdAt: state.tables.find((table) => table.id === input.tableId)?.createdAt ?? timestamp,
            updatedAt: timestamp
          },
          input.description
        )
      )
    );

    const tables = input.tableId
      ? state.tables.map((table) => (table.id === input.tableId ? nextTable : table))
      : [...state.tables, nextTable];

    writeStoredState({ tables });

    return Promise.resolve({
      table: nextTable,
      importedRowCount: rows.length,
      format: input.format
    });
  },

  exportTable(id: string, format: "csv" | "json"): Promise<DataTableExportDto> {
    const table = readStoredState().tables.find((item) => item.id === id);
    if (!table) {
      throw new Error("Không tìm thấy bảng preview để export.");
    }

    return Promise.resolve({
      fileName: `${table.name.trim().toLowerCase().replace(/\s+/g, "-") || "data-table"}.${format}`,
      format,
      content: format === "csv" ? toCsv(table) : toJson(table),
      table
    });
  }
};
