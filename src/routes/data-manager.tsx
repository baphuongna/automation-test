import { useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import { dataTableClient, getDataTablePreviewBanner } from "../services/data-table-client";
import { useDataTableStore } from "../store/data-table-store";
import type { DataTableColumnDto, DataTableDto } from "../types";

interface DataTableFormState {
  id: string | null;
  name: string;
  description: string;
  columnsText: string;
}

interface DataTableRowFormState {
  id: string | null;
  values: string[];
  enabled: boolean;
  rowIndex: number;
}

interface ImportFormState {
  format: "csv" | "json";
  content: string;
}

const EMPTY_TABLE_FORM: DataTableFormState = {
  id: null,
  name: "",
  description: "",
  columnsText: ""
};

const EMPTY_IMPORT_FORM: ImportFormState = {
  format: "csv",
  content: ""
};

function createTableSummary(table: DataTableDto) {
  return {
    id: table.id,
    name: table.name,
    totalRowCount: table.associationMeta.totalRowCount,
    enabledRowCount: table.associationMeta.enabledRowCount
  };
}

function parseColumnsText(columnsText: string): DataTableColumnDto[] {
  return columnsText
    .split(",")
    .map((column) => column.trim())
    .filter((column) => column.length > 0)
    .map((column) => ({ name: column, colType: "string" }));
}

function createEmptyRowForm(columnCount: number, rowIndex: number): DataTableRowFormState {
  return {
    id: null,
    values: Array.from({ length: columnCount }, () => ""),
    enabled: true,
    rowIndex
  };
}

function createColumnsText(columns: DataTableColumnDto[]): string {
  return columns.map((column) => column.name).join(", ");
}

function normalizeDescription(description: string): string | undefined {
  const trimmed = description.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function buildTableSaveInput(form: DataTableFormState, columns: DataTableColumnDto[]) {
  const description = normalizeDescription(form.description);

  return description === undefined
    ? {
        name: form.name.trim(),
        columns
      }
    : {
        name: form.name.trim(),
        description,
        columns
      };
}

function buildImportInput(
  form: DataTableFormState,
  importForm: ImportFormState,
  activeTable: DataTableDto | null,
  name: string
) {
  const description = normalizeDescription(form.description);

  return {
    name,
    format: importForm.format,
    content: importForm.content,
    ...(activeTable?.id ? { tableId: activeTable.id } : {}),
    ...(description === undefined ? {} : { description })
  };
}

export default function DataManager() {
  const { activeTableId, setActiveTableId, setTables } = useDataTableStore();
  const [tables, setTableRecords] = useState<DataTableDto[]>([]);
  const [tableForm, setTableForm] = useState<DataTableFormState>(EMPTY_TABLE_FORM);
  const [rowForm, setRowForm] = useState<DataTableRowFormState>(createEmptyRowForm(1, 0));
  const [importForm, setImportForm] = useState<ImportFormState>(EMPTY_IMPORT_FORM);
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingTable, setIsSavingTable] = useState(false);
  const [isSavingRow, setIsSavingRow] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [exportContent, setExportContent] = useState<string>("");
  const previewBanner = getDataTablePreviewBanner();

  const activeTable = useMemo(
    () => tables.find((table) => table.id === activeTableId) ?? tables[0] ?? null,
    [activeTableId, tables]
  );

  useEffect(() => {
    void loadTables();
  }, []);

  useEffect(() => {
    if (!activeTable) {
      setTableForm(EMPTY_TABLE_FORM);
      setRowForm(createEmptyRowForm(1, 0));
      return;
    }

    setTableForm({
      id: activeTable.id,
      name: activeTable.name,
      description: activeTable.description ?? "",
      columnsText: createColumnsText(activeTable.columns)
    });

    setRowForm(createEmptyRowForm(activeTable.columns.length || 1, activeTable.rows.length));
  }, [activeTable]);

  async function loadTables(): Promise<void> {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const records = await dataTableClient.list();
      setTableRecords(records);
      setTables(records.map(createTableSummary));

      const nextActiveId = records.find((table) => table.id === activeTableId)?.id ?? records[0]?.id ?? null;
      setActiveTableId(nextActiveId);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể tải danh sách bảng dữ liệu.";
      setErrorMessage(message);
    } finally {
      setIsLoading(false);
    }
  }

  async function handleTableSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    const columns = parseColumnsText(tableForm.columnsText);
    if (tableForm.name.trim().length === 0) {
      setErrorMessage("Table name is required.");
      return;
    }

    if (columns.length === 0) {
      setErrorMessage("At least one column is required.");
      return;
    }

    setIsSavingTable(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      const tablePayload = buildTableSaveInput(tableForm, columns);

      const saved = tableForm.id
        ? await dataTableClient.update({
            id: tableForm.id,
            ...tablePayload
          })
        : await dataTableClient.create(tablePayload);

      await loadTables();
      setActiveTableId(saved.id);
      setFeedbackMessage(tableForm.id ? "Data table updated." : "Data table created.");
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu bảng dữ liệu.";
      setErrorMessage(message);
    } finally {
      setIsSavingTable(false);
    }
  }

  function handleCreateTableDraft(): void {
    setActiveTableId(null);
    setTableForm(EMPTY_TABLE_FORM);
    setRowForm(createEmptyRowForm(1, 0));
    setFeedbackMessage(null);
    setErrorMessage(null);
    setExportContent("");
  }

  async function handleDeleteTable(): Promise<void> {
    if (!activeTable) {
      return;
    }

    if (!window.confirm(`Delete data table '${activeTable.name}' and all rows?`)) {
      return;
    }

    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      await dataTableClient.remove(activeTable.id);
      setFeedbackMessage("Data table deleted.");
      setExportContent("");
      await loadTables();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa bảng dữ liệu.";
      setErrorMessage(message);
    }
  }

  async function handleRowSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    if (!activeTable) {
      setErrorMessage("Chọn một bảng trước khi lưu dòng dữ liệu.");
      return;
    }

    if (rowForm.values.length !== activeTable.columns.length) {
      setErrorMessage("Row value count must match the number of columns.");
      return;
    }

    setIsSavingRow(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      await dataTableClient.upsertRow({
        tableId: activeTable.id,
        row: {
          values: rowForm.values,
          enabled: rowForm.enabled,
          rowIndex: rowForm.rowIndex,
          ...(rowForm.id ? { id: rowForm.id } : {})
        }
      });

      setFeedbackMessage(rowForm.id ? "Row updated." : "Row created.");
      setRowForm(createEmptyRowForm(activeTable.columns.length, activeTable.rows.length + (rowForm.id ? 0 : 1)));
      await loadTables();
      setActiveTableId(activeTable.id);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể lưu dòng dữ liệu.";
      setErrorMessage(message);
    } finally {
      setIsSavingRow(false);
    }
  }

  async function handleDeleteRow(rowId: string): Promise<void> {
    if (!activeTable) {
      return;
    }

    if (!window.confirm("Delete selected row?")) {
      return;
    }

    setErrorMessage(null);
    setFeedbackMessage(null);

    try {
      await dataTableClient.deleteRow(rowId);
      setFeedbackMessage("Row deleted.");
      await loadTables();
      setActiveTableId(activeTable.id);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể xóa dòng dữ liệu.";
      setErrorMessage(message);
    }
  }

  function handleEditRow(rowId: string): void {
    if (!activeTable) {
      return;
    }

    const row = activeTable.rows.find((item) => item.id === rowId);
    if (!row) {
      return;
    }

    setRowForm({
      id: row.id,
      values: [...row.values],
      enabled: row.enabled,
      rowIndex: row.rowIndex
    });
    setFeedbackMessage(null);
    setErrorMessage(null);
  }

  async function handleImportSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    const name = tableForm.name.trim() || (activeTable?.name ?? "Imported Table");
    setIsImporting(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      const result = await dataTableClient.importTable(buildImportInput(tableForm, importForm, activeTable, name));

      await loadTables();
      setActiveTableId(result.table.id);
      setFeedbackMessage(`Imported ${result.importedRowCount} rows from ${result.format.toUpperCase()}.`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Import failed due to malformed content.";
      setErrorMessage(`Import failed: ${message}`);
    } finally {
      setIsImporting(false);
    }
  }

  async function handleExport(format: "csv" | "json"): Promise<void> {
    if (!activeTable) {
      return;
    }

    setIsExporting(true);
    setFeedbackMessage(null);
    setErrorMessage(null);

    try {
      const exported = await dataTableClient.exportTable(activeTable.id, format);
      setExportContent(exported.content);
      setFeedbackMessage(`Exported ${format.toUpperCase()} baseline for ${exported.fileName}.`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không thể export bảng dữ liệu.";
      setErrorMessage(message);
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <section className="data-manager" data-testid="route-data-manager">
      <header className="data-manager__hero">
        <div>
          <span className="route-skeleton__eyebrow">Data tables</span>
          <h1>Data Manager</h1>
          <p>
            Maintain reusable data table CRUD, row editing, import/export baseline, and association metadata
            exposure for future test-case linkage.
          </p>
        </div>
        <button className="data-manager__primary-action" type="button" onClick={handleCreateTableDraft}>
          New table
        </button>
      </header>

      {previewBanner ? <div className="data-manager__feedback">{previewBanner}</div> : null}
      {feedbackMessage ? <div className="data-manager__feedback">{feedbackMessage}</div> : null}
      {errorMessage ? <div className="data-manager__feedback data-manager__feedback--error">{errorMessage}</div> : null}

      <div className="data-manager__layout">
        <section className="data-panel data-panel--list">
          <div className="data-panel__header">
            <h2>Tables</h2>
            <span>{tables.length} total</span>
          </div>

          {isLoading ? <p className="data-panel__empty">Loading data tables...</p> : null}
          {!isLoading && tables.length === 0 ? (
            <p className="data-panel__empty">empty table state: create your first data table.</p>
          ) : null}

          <div className="data-list">
            {tables.map((table) => (
              <button
                key={table.id}
                className={`data-list__item${table.id === activeTable?.id ? " data-list__item--active" : ""}`}
                type="button"
                onClick={() => {
                  setActiveTableId(table.id);
                  setFeedbackMessage(null);
                  setErrorMessage(null);
                  setExportContent("");
                }}
              >
                <div className="data-list__name-row">
                  <strong>{table.name}</strong>
                  <span className="data-badge">{table.associationMeta.enabledRowCount} enabled</span>
                </div>
                <p>
                  {table.columns.length} columns · {table.associationMeta.totalRowCount} rows
                </p>
              </button>
            ))}
          </div>
        </section>

        <section className="data-panel">
          <div className="data-panel__header">
            <h2>Table form</h2>
            {activeTable ? <span>{activeTable.id}</span> : <span>Create draft</span>}
          </div>

          <form className="data-form" onSubmit={handleTableSubmit}>
            <label className="data-field">
              <span>Name</span>
              <input
                value={tableForm.name}
                onChange={(event) => setTableForm((current) => ({ ...current, name: event.target.value }))}
              />
            </label>

            <label className="data-field">
              <span>Description</span>
              <textarea
                rows={3}
                value={tableForm.description}
                onChange={(event) => setTableForm((current) => ({ ...current, description: event.target.value }))}
              />
            </label>

            <label className="data-field">
              <span>Columns (comma separated)</span>
              <input
                value={tableForm.columnsText}
                onChange={(event) => setTableForm((current) => ({ ...current, columnsText: event.target.value }))}
              />
            </label>

            <div className="data-form__actions">
              <button disabled={isSavingTable} type="submit">
                {tableForm.id ? "Save table" : "Create table"}
              </button>
              <button className="data-button--ghost" type="button" onClick={handleCreateTableDraft}>
                Reset draft
              </button>
              <button
                className="data-button--danger"
                disabled={!activeTable}
                type="button"
                onClick={() => void handleDeleteTable()}
              >
                Delete table
              </button>
            </div>
          </form>

          <div className="data-association-card">
            <div className="data-panel__header">
              <h2>association metadata</h2>
              <span>association-ready</span>
            </div>
            {activeTable ? (
              <>
                <p>
                  Future test-case association can rely on a stable table id plus row counts without implementing
                  linkage UI in T7.
                </p>
                <dl className="data-meta-grid">
                  <div>
                    <dt>Can associate</dt>
                    <dd>{activeTable.associationMeta.canAssociateToTestCases ? "Yes" : "No"}</dd>
                  </div>
                  <div>
                    <dt>Enabled rows</dt>
                    <dd>{activeTable.associationMeta.enabledRowCount}</dd>
                  </div>
                  <div>
                    <dt>Total rows</dt>
                    <dd>{activeTable.associationMeta.totalRowCount}</dd>
                  </div>
                  <div>
                    <dt>Linked test cases</dt>
                    <dd>{activeTable.associationMeta.linkedTestCaseIds.length}</dd>
                  </div>
                </dl>
              </>
            ) : (
              <p className="data-panel__empty">Create or select a table to inspect association metadata.</p>
            )}
          </div>
        </section>

        <section className="data-panel">
          <div className="data-panel__header">
            <h2>Rows and import/export</h2>
            {activeTable ? <span>{activeTable.columns.length} columns</span> : <span>No active table</span>}
          </div>

          {activeTable ? (
            <>
              {activeTable.rows.length === 0 ? <p className="data-panel__empty">This empty table has no rows yet.</p> : null}
              {activeTable.associationMeta.enabledRowCount === 0 ? (
                <div className="data-manager__feedback data-manager__feedback--warning">
                  zero enabled rows: this table will be skipped by future association-driven runs until at least one row
                  is enabled.
                </div>
              ) : null}

              <div className="data-row-list">
                {activeTable.rows.map((row) => (
                  <article
                    key={row.id}
                    className={`data-row-card${row.enabled ? "" : " data-row-card--disabled"}`}
                  >
                    <div>
                      <div className="data-list__name-row">
                        <strong>Row #{row.rowIndex + 1}</strong>
                        <span className={`data-badge${row.enabled ? "" : " data-badge--warning"}`}>
                          {row.enabled ? "Enabled" : "Disabled"}
                        </span>
                      </div>
                      <p>{row.values.join(" | ") || "(empty row)"}</p>
                    </div>
                    <div className="data-form__actions">
                      <button className="data-button--ghost" type="button" onClick={() => handleEditRow(row.id)}>
                        Edit
                      </button>
                      <button className="data-button--danger" type="button" onClick={() => void handleDeleteRow(row.id)}>
                        Delete
                      </button>
                    </div>
                  </article>
                ))}
              </div>

              <form className="data-form" onSubmit={handleRowSubmit}>
                {activeTable.columns.map((column, index) => (
                  <label key={`${column.name}-${index}`} className="data-field">
                    <span>{column.name}</span>
                    <input
                      value={rowForm.values[index] ?? ""}
                      onChange={(event) =>
                        setRowForm((current) => {
                          const values = [...current.values];
                          values[index] = event.target.value;
                          return { ...current, values };
                        })
                      }
                    />
                  </label>
                ))}

                <label className="data-toggle">
                  <input
                    checked={rowForm.enabled}
                    type="checkbox"
                    onChange={(event) => setRowForm((current) => ({ ...current, enabled: event.target.checked }))}
                  />
                  <span>Enabled for execution</span>
                </label>

                <div className="data-form__actions">
                  <button disabled={isSavingRow} type="submit">
                    {rowForm.id ? "Save row" : "Add row"}
                  </button>
                  <button
                    className="data-button--ghost"
                    type="button"
                    onClick={() =>
                      setRowForm(createEmptyRowForm(activeTable.columns.length, activeTable.rows.length))
                    }
                  >
                    Reset row form
                  </button>
                </div>
              </form>

              <form className="data-form" onSubmit={handleImportSubmit}>
                <label className="data-field">
                  <span>Import format</span>
                  <select
                    value={importForm.format}
                    onChange={(event) =>
                      setImportForm((current) => ({ ...current, format: event.target.value as "csv" | "json" }))
                    }
                  >
                    <option value="csv">CSV</option>
                    <option value="json">JSON</option>
                  </select>
                </label>

                <label className="data-field">
                  <span>Import content</span>
                  <textarea
                    rows={8}
                    value={importForm.content}
                    onChange={(event) => setImportForm((current) => ({ ...current, content: event.target.value }))}
                    placeholder="Paste CSV or JSON baseline here. Malformed import must be rejected without partial corruption."
                  />
                </label>

                <div className="data-form__actions">
                  <button disabled={isImporting} type="submit">
                    Import baseline
                  </button>
                  <button disabled={isExporting} type="button" onClick={() => void handleExport("csv")}>
                    Export CSV
                  </button>
                  <button disabled={isExporting} type="button" onClick={() => void handleExport("json")}>
                    Export JSON
                  </button>
                </div>
              </form>

              <label className="data-field">
                <span>Export preview</span>
                <textarea readOnly rows={8} value={exportContent} />
              </label>
            </>
          ) : (
            <p className="data-panel__empty">Select a table to manage rows, import baseline, and export baseline.</p>
          )}
        </section>
      </div>
    </section>
  );
}
