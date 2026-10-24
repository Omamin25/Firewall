import { create } from "zustand";
import {
  createRule,
  deleteRule,
  getEnforcementStatus,
  listRules,
  toggleRule,
  updateRule,
} from "@/lib/ipc";
import type { EnforcementStatus, Rule, RuleDraft } from "@/types/ipc";

export type RulesState = {
  rules: Rule[];
  enforcement: EnforcementStatus | null;
  /** True until the first load settles, so the table can skeleton. */
  loading: boolean;
  /** Load/mutation failure for the page-level ErrorState. Form errors stay in the form. */
  error: string | null;
  /** Rule ids with a mutation in flight, so their row controls disable individually. */
  pending: number[];
  refresh: () => Promise<void>;
  /** Just the engine state — used by the status pill and the overview, which do not need
   *  the rule rows and should not pay for a database read every few seconds. */
  refreshEnforcement: () => Promise<void>;
  /** Creates when `ruleId` is null, otherwise updates. Rejects with the Rust message. */
  save: (ruleId: number | null, draft: RuleDraft) => Promise<Rule>;
  toggle: (ruleId: number, enabled: boolean) => Promise<void>;
  remove: (ruleId: number) => Promise<void>;
};

export const useRulesStore = create<RulesState>((set, get) => ({
  rules: [],
  enforcement: null,
  loading: true,
  error: null,
  pending: [],

  refresh: async () => {
    try {
      // Enforcement state is fetched alongside the rules so the summary strip and the
      // table can never disagree about how many rules are live.
      const [rules, enforcement] = await Promise.all([listRules(), getEnforcementStatus()]);
      set({ rules: sortRules(rules), enforcement, loading: false, error: null });
    } catch (cause: unknown) {
      set({ loading: false, error: message(cause, "Rules could not be loaded.") });
    }
  },

  refreshEnforcement: async () => {
    try {
      set({ enforcement: await getEnforcementStatus() });
    } catch {
      // A failed poll leaves the last known state rather than blanking the pill; the
      // engine's own error field is how real failures surface.
    }
  },

  save: async (ruleId, draft) => {
    const saved = ruleId === null ? await createRule(draft) : await updateRule(ruleId, draft);
    set((state) => ({
      rules: sortRules([...state.rules.filter((rule) => rule.id !== saved.id), saved]),
      error: null,
    }));
    // The engine reconciled before the command returned; pull the new counters.
    void get().refresh();
    return saved;
  },

  toggle: async (ruleId, enabled) => {
    set((state) => ({ pending: [...state.pending, ruleId] }));
    try {
      const saved = await toggleRule(ruleId, enabled);
      set((state) => ({
        rules: sortRules(state.rules.map((rule) => (rule.id === saved.id ? saved : rule))),
        error: null,
      }));
      void get().refresh();
    } finally {
      set((state) => ({ pending: state.pending.filter((id) => id !== ruleId) }));
    }
  },

  remove: async (ruleId) => {
    set((state) => ({ pending: [...state.pending, ruleId] }));
    try {
      await deleteRule(ruleId);
      set((state) => ({ rules: state.rules.filter((rule) => rule.id !== ruleId), error: null }));
      void get().refresh();
    } finally {
      set((state) => ({ pending: state.pending.filter((id) => id !== ruleId) }));
    }
  },
}));

/** Evaluation order, so the table reads the way the engine decides: priority first, then
 *  allow before block at equal priority, then id (`rules/compiled.rs`). */
export function sortRules(rules: readonly Rule[]): Rule[] {
  return [...rules].sort((left, right) => {
    if (left.priority !== right.priority) return left.priority - right.priority;
    if (left.action !== right.action) return left.action === "allow" ? -1 : 1;
    return left.id - right.id;
  });
}

function message(cause: unknown, fallback: string): string {
  if (typeof cause === "string") return cause;
  if (cause instanceof Error) return cause.message;
  return fallback;
}
