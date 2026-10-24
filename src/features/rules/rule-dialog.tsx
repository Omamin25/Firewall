import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Modal } from "@/components/ui/modal";
import { RuleForm } from "@/features/rules/rule-form";
import { formValuesFromRule, type RuleFormValues } from "@/features/rules/rule-criteria";
import { useRuleForm } from "@/features/rules/use-rule-form";
import { useRulesStore } from "@/store/use-rules-store";
import { toastSuccess } from "@/store/use-toast-store";
import type { Rule } from "@/types/ipc";

const FORM_ID = "rule-form";

export type RuleDialogProps = {
  /** The rule being edited, or null when creating a new one. */
  rule: Rule | null;
  /** Prefilled criteria — from a connection or a packet. Ignored when `rule` is set. */
  initial?: RuleFormValues;
  /** Explains where prefilled criteria came from. */
  description?: string;
  onClose: () => void;
};

/** Create/edit rule dialog. Mounted only while open, so the form always starts from the
 *  values it was given (`ui_rules.md` §8 — modals are for this and destructive confirms). */
export function RuleDialog({ rule, initial, description, onClose }: RuleDialogProps): JSX.Element {
  const form = useRuleForm(rule ? formValuesFromRule(rule) : initial);
  const save = useRulesStore((state) => state.save);
  const [submitting, setSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const editing = rule !== null;

  const submit = (): void => {
    form.touchAll();
    if (!form.valid || form.draft === null || submitting) return;
    setSubmitting(true);
    setServerError(null);
    save(editing ? rule.id : null, form.draft)
      .then((saved) => {
        toastSuccess(
          editing ? `Rule "${saved.name}" updated.` : `Rule "${saved.name}" created.`,
        );
        onClose();
      })
      .catch((cause: unknown) => {
        setServerError(
          typeof cause === "string"
            ? cause
            : cause instanceof Error
              ? cause.message
              : "The rule could not be saved.",
        );
      })
      .finally(() => setSubmitting(false));
  };

  return (
    <Modal
      open
      onClose={onClose}
      title={editing ? "Edit rule" : "New rule"}
      description={description}
      dismissible={!submitting}
      footer={
        <>
          <Button variant="outline" size="sm" onClick={onClose} disabled={submitting}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={FORM_ID}
            variant="accent"
            size="sm"
            disabled={!form.valid || submitting}
          >
            {submitting ? "Saving…" : editing ? "Save rule" : "Create rule"}
          </Button>
        </>
      }
    >
      <RuleForm form={form} formId={FORM_ID} serverError={serverError} onSubmit={submit} />
    </Modal>
  );
}
