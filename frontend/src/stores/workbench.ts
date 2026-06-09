import { defineStore } from "pinia";
import type { TaskFilters } from "@/types";

export type WorkbenchView = "kanban" | "gantt" | "list";

export const useWorkbenchStore = defineStore("workbench", {
  state: () => ({
    view: "kanban" as WorkbenchView,
    filters: {
      project_id: "project-1",
    } as TaskFilters,
  }),
  actions: {
    setView(view: WorkbenchView) {
      this.view = view;
    },
    setFilters(filters: TaskFilters) {
      this.filters = { ...filters };
    },
    clearFilter(key: keyof TaskFilters) {
      const next = { ...this.filters };
      delete next[key];
      this.filters = next;
    },
    resetFilters() {
      this.filters = {};
    },
  },
});
