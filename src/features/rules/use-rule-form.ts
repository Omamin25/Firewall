import { useCallback, useMemo, useState } from "react";
import {
  EMPTY_RULE_FORM,
  draftFromFormValues,
  validateRuleForm,
  type RuleFormErrors,
  type RuleFormField,
  type RuleFormValues,
} from "@/features/rules/rule-criteria";
import type { RuleDraft } from "@/types/ipc";

export type RuleFormController = {
  values: RuleFormValues;
  /** Only the fields the user has left or tried to submit — never blocks typing. */
  visibleErrors: RuleFormErrors;
  valid: boolean;
  draft: RuleDraft | null;
  setField: <F extends RuleFormField>(field: F, value: RuleFormValues[F]) => void;
  markTouched: (field: RuleFormField) => void;
  /** Reveals every message; call before rejecting a submit. */
  touchAll: () => void;
};

/** Form state for the create/edit rule dialog (`ui_rules.md` §4: validate on blur and
 *  submit, disable submit until valid). Rust validates again and has the last word. */
export function useRuleForm(initial: RuleFormValues = EMPTY_RULE_FORM): RuleFormController {
  const [values, setValues] = useState<RuleFormValues>(initial);
  const [touched, setTouched] = useState<ReadonlySet<RuleFormField>>(new Set());

  const errors = useMemo(() => validateRuleForm(values), [values]);
  const draft = useMemo(() => draftFromFormValues(values), [values]);

  const visibleErrors = useMemo(() => {
    const visible: RuleFormErrors = {};
    for (const [field, message] of Object.entries(errors) as [RuleFormField, string][]) {
      if (touched.has(field)) visible[field] = message;
    }
    return visible;
  }, [errors, touched]);

  const setField = useCallback(
    <F extends RuleFormField>(field: F, value: RuleFormValues[F]) => {
      setValues((current) => ({ ...current, [field]: value }));
    },
    [],
  );

  const markTouched = useCallback((field: RuleFormField) => {
    setTouched((current) => (current.has(field) ? current : new Set(current).add(field)));
  }, []);

  const touchAll = useCallback(() => {
    setTouched(new Set(Object.keys(values) as RuleFormField[]));
  }, [values]);

  return {
    values,
    visibleErrors,
    valid: Object.keys(errors).length === 0,
    draft,
    setField,
    markTouched,
    touchAll,
  };
}
