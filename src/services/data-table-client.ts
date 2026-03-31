import type { CommandError } from "./tauri-client";
import { invokeCommand } from "./tauri-client";
import { dataTablePreviewClient } from "./data-table-preview-client";
import type { CommandName, CommandPayloadMap, CommandResponseMap, DataTableDto } from "../types";

export const DATA_TABLE_PREVIEW_FALLBACK_BANNER = "Preview fallback active - browser-only T7 verification path.";

function isTauriRuntimeAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function shouldUsePreviewFallback(): boolean {
  return !isTauriRuntimeAvailable() && dataTablePreviewClient.isAvailable();
}

function createFallbackError(command: CommandName): CommandError {
  return {
    code: "INTERNAL_UNEXPECTED_ERROR",
    context: { command },
    displayMessage: `Không thể thực hiện lệnh ${command}.`,
    recoverable: false,
    technicalMessage: `Missing command result for ${command}`
  };
}

async function unwrapCommand<TName extends CommandName>(
  command: TName,
  payload: CommandPayloadMap[TName]
): Promise<CommandResponseMap[TName]> {
  const result = await invokeCommand(command, payload);

  if (!result.success || result.data === null) {
    throw result.error ?? createFallbackError(command);
  }

  return result.data;
}

export interface DataTableSaveInput {
  id?: string;
  name: string;
  description?: string;
  columns: DataTableDto["columns"];
}

export interface DataTableRowSaveInput {
  tableId: string;
  row: {
    id?: string;
    values: string[];
    enabled: boolean;
    rowIndex: number;
  };
}

export interface DataTableImportInput {
  tableId?: string;
  name: string;
  description?: string;
  format: "csv" | "json";
  content: string;
}

function withOptionalDescription<T extends { description?: string }>(
  payload: Omit<T, "description">,
  description: string | undefined
): T {
  if (description === undefined) {
    return payload as T;
  }

  return {
    ...payload,
    description
  } as T;
}

export const dataTableClient = {
  list(): Promise<CommandResponseMap["dataTable.list"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.list();
    }

    return unwrapCommand("dataTable.list", {});
  },

  create(input: DataTableSaveInput): Promise<CommandResponseMap["dataTable.create"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.create(input);
    }

    return unwrapCommand(
      "dataTable.create",
      withOptionalDescription<CommandPayloadMap["dataTable.create"]>(
        {
          name: input.name,
          columns: input.columns
        },
        input.description
      )
    );
  },

  update(input: DataTableSaveInput & { id: string }): Promise<CommandResponseMap["dataTable.update"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.update(input);
    }

    return unwrapCommand(
      "dataTable.update",
      withOptionalDescription<CommandPayloadMap["dataTable.update"]>(
        {
          id: input.id,
          name: input.name,
          columns: input.columns
        },
        input.description
      )
    );
  },

  remove(id: string): Promise<CommandResponseMap["dataTable.delete"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.remove(id);
    }

    return unwrapCommand("dataTable.delete", { id });
  },

  upsertRow(input: DataTableRowSaveInput): Promise<CommandResponseMap["dataTable.row.upsert"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.upsertRow(input);
    }

    return unwrapCommand("dataTable.row.upsert", {
      tableId: input.tableId,
      row: {
        id: input.row.id ?? "",
        values: input.row.values,
        enabled: input.row.enabled,
        rowIndex: input.row.rowIndex
      }
    });
  },

  deleteRow(id: string): Promise<CommandResponseMap["dataTable.row.delete"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.deleteRow(id);
    }

    return unwrapCommand("dataTable.row.delete", { id });
  },

  importTable(input: DataTableImportInput): Promise<CommandResponseMap["dataTable.import"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.importTable(input);
    }

    const payload: CommandPayloadMap["dataTable.import"] = {
      name: input.name,
      format: input.format,
      content: input.content,
      ...(input.tableId ? { tableId: input.tableId } : {}),
      ...(input.description !== undefined ? { description: input.description } : {})
    };

    return unwrapCommand("dataTable.import", payload);
  },

  exportTable(id: string, format: "csv" | "json"): Promise<CommandResponseMap["dataTable.export"]> {
    if (shouldUsePreviewFallback()) {
      return dataTablePreviewClient.exportTable(id, format);
    }

    return unwrapCommand("dataTable.export", { id, format });
  }
};

export function getDataTablePreviewBanner(): string | null {
  return shouldUsePreviewFallback() ? DATA_TABLE_PREVIEW_FALLBACK_BANNER : null;
}
