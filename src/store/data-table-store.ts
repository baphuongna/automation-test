import { create } from "zustand";

export interface DataTableSummary {
  id: string;
  name: string;
  totalRowCount: number;
  enabledRowCount: number;
}

interface DataTableState {
  activeTableId: string | null;
  tables: DataTableSummary[];
  setTables: (tables: DataTableSummary[]) => void;
  setActiveTableId: (tableId: string | null) => void;
  reset: () => void;
}

const initialDataTableState = {
  activeTableId: null,
  tables: [] as DataTableSummary[]
};

export const useDataTableStore = create<DataTableState>((set) => ({
  ...initialDataTableState,
  setTables: (tables) => {
    set((state) => {
      const activeTableExists = tables.some((table) => table.id === state.activeTableId);

      return {
        activeTableId: activeTableExists ? state.activeTableId : (tables[0]?.id ?? null),
        tables
      };
    });
  },
  setActiveTableId: (tableId) => {
    set({ activeTableId: tableId });
  },
  reset: () => {
    set(initialDataTableState);
  }
}));
