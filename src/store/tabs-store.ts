import { create } from "zustand";

export interface AppTab {
  id: string;
  title: string;
  route: string;
}

interface TabsState {
  activeTabId: string | null;
  tabs: AppTab[];
  openTab: (tab: AppTab) => void;
  closeTab: (tabId: string) => void;
  setActiveTabId: (tabId: string | null) => void;
  reset: () => void;
}

const initialTabsState = {
  activeTabId: null,
  tabs: [] as AppTab[]
};

export const useTabsStore = create<TabsState>((set) => ({
  ...initialTabsState,
  openTab: (tab) => {
    set((state) => {
      const hasTab = state.tabs.some((item) => item.id === tab.id);

      return {
        activeTabId: tab.id,
        tabs: hasTab ? state.tabs : [...state.tabs, tab]
      };
    });
  },
  closeTab: (tabId) => {
    set((state) => {
      const nextTabs = state.tabs.filter((tab) => tab.id !== tabId);
      const nextActiveTabId =
        state.activeTabId === tabId ? (nextTabs.at(-1)?.id ?? null) : state.activeTabId;

      return {
        activeTabId: nextActiveTabId,
        tabs: nextTabs
      };
    });
  },
  setActiveTabId: (tabId) => {
    set({ activeTabId: tabId });
  },
  reset: () => {
    set(initialTabsState);
  }
}));
