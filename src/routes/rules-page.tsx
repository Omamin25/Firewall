import { useEffect, useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { ErrorState } from "@/components/ui/error-state";
import { EnforcementSummary } from "@/features/rules/enforcement-summary";
import { RuleDialog } from "@/features/rules/rule-dialog";
import { RulesTable } from "@/features/rules/rules-table";
import { useRulesStore } from "@/store/use-rules-store";
import { toastError, toastSuccess } from "@/store/use-toast-store";
import type { Rule } from "@/types/ipc";

/** Hit counts and dropped-packet totals move while the page is open, so they are polled.
 *  Every mutation already refreshes on its own. */
const REFRESH_MS = 5_000;

type Dialog =
  | { kind: "closed" }
  | { kind: "create" }
  | { kind: "edit"; rule: Rule }
  | { kind: "confirm-delete"; rule: Rule };

export function RulesPage(): JSX.Element {
  const rules = useRulesStore((state) => state.rules);
  const enforcement = useRulesStore((state) => state.enforcement);
  const loading = useRulesStore((state) => state.loading);
  const error = useRulesStore((state) => state.error);
  const refresh = useRulesStore((state) => state.refresh);
  const remove = useRulesStore((state) => state.remove);
  const [dialog, setDialog] = useState<Dialog>({ kind: "closed" });
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const confirmDelete = (rule: Rule): void => {
    setDeleting(true);
    remove(rule.id)
      .then(() => {
        toastSuccess(`Rule "${rule.name}" deleted.`);
        setDialog({ kind: "closed" });
      })
      .catch((cause: unknown) =>
        toastError(typeof cause === "string" ? cause : "The rule could not be deleted."),
      )
      .finally(() => setDeleting(false));
  };

  return (
    <div className="space-y-4">
      {error ? <ErrorState title="Rules unavailable" detail={error} /> : null}

      <Card
        title="Firewall rules"
        action={
          <div className="flex items-center gap-4">
            <EnforcementSummary enforcement={enforcement} />
            <Button
              variant="accent"
              size="sm"
              icon={<Plus aria-hidden className="size-3.5" />}
              onClick={() => setDialog({ kind: "create" })}
            >
              Add rule
            </Button>
          </div>
        }
      >
        <div className="space-y-3">
          <p className="text-xs text-foreground-muted">
            Rules are evaluated top to bottom: lower priority first, and an allow beats a
            block at the same priority. Only enabled rules are matched.
          </p>
          <RulesTable
            rules={rules}
            loading={loading}
            onEdit={(rule) => setDialog({ kind: "edit", rule })}
            onDelete={(rule) => setDialog({ kind: "confirm-delete", rule })}
          />
        </div>
      </Card>

      {dialog.kind === "create" ? (
        <RuleDialog rule={null} onClose={() => setDialog({ kind: "closed" })} />
      ) : null}
      {dialog.kind === "edit" ? (
        <RuleDialog rule={dialog.rule} onClose={() => setDialog({ kind: "closed" })} />
      ) : null}
      {dialog.kind === "confirm-delete" ? (
        <ConfirmDialog
          open
          title="Delete this rule?"
          detail={`"${dialog.rule.name}" will be removed from the database and stop being enforced immediately. This cannot be undone — disable it instead if you only want to pause it.`}
          confirmLabel="Delete rule"
          busy={deleting}
          onConfirm={() => confirmDelete(dialog.rule)}
          onCancel={() => setDialog({ kind: "closed" })}
        />
      ) : null}
    </div>
  );
}
