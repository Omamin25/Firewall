import { useEffect, useState } from "react";
import { Ban, ShieldCheck } from "lucide-react";
import { Menu, type MenuEntry } from "@/components/ui/menu";
import { formValuesFromDraft, type RuleFormValues } from "@/features/rules/rule-criteria";
import { RuleDialog } from "@/features/rules/rule-dialog";
import { toastError } from "@/store/use-toast-store";
import type { RuleAction, RuleDraft, RuleScope } from "@/types/ipc";

export type ScopeChoice = {
  scope: RuleScope;
  action: RuleAction;
  label: string;
  /** False when the traffic lacks what the scope names (e.g. an unresolved process). */
  enabled?: boolean;
};

export type CreateRuleFromTrafficProps = {
  /** Viewport point the menu opens at: a button rect or a right-click position. */
  anchor: { x: number; y: number };
  /** What the rule is being derived from, e.g. `example.com:443`. */
  subject: string;
  scopes: readonly ScopeChoice[];
  /** Asks Rust for prefilled criteria — the UI never derives them itself. */
  loadDraft: (scope: RuleScope, action: RuleAction) => Promise<RuleDraft>;
  /** Copy actions appended under a second heading. */
  extraItems?: readonly MenuEntry[];
  /** True when the menu opens from inside the inspector drawer, which sits above the
   *  dropdown layer. */
  elevated?: boolean;
  onClose: () => void;
};

/** Shared machinery behind "create a rule from this": a scope menu, a round trip to Rust
 *  for the criteria, then the normal rule dialog with the fields already filled. Nothing
 *  is written until the user confirms, so blocking stays explicit (`architecture.md` §5). */
export function CreateRuleFromTraffic({
  anchor,
  subject,
  scopes,
  loadDraft,
  extraItems = [],
  elevated = false,
  onClose,
}: CreateRuleFromTrafficProps): JSX.Element | null {
  const [menuOpen, setMenuOpen] = useState(true);
  const [values, setValues] = useState<RuleFormValues | null>(null);
  const [loading, setLoading] = useState(false);

  // Dismissing the menu without choosing anything ends the whole interaction.
  useEffect(() => {
    if (!menuOpen && !loading && values === null) onClose();
  }, [menuOpen, loading, values, onClose]);

  const choose = (scope: RuleScope, action: RuleAction): void => {
    setLoading(true);
    loadDraft(scope, action)
      .then((draft) => setValues(formValuesFromDraft(draft)))
      .catch((cause: unknown) => {
        toastError(
          typeof cause === "string" ? cause : "Criteria for this traffic could not be read.",
        );
        onClose();
      })
      .finally(() => setLoading(false));
  };

  if (values !== null) {
    return (
      <RuleDialog
        rule={null}
        initial={values}
        description={`Prefilled from ${subject}. Widen or narrow anything before saving.`}
        onClose={onClose}
      />
    );
  }

  const items: MenuEntry[] = [
    { kind: "label", label: "Create rule" },
    ...scopes.map((choice) => ({
      kind: "action" as const,
      label: choice.label,
      icon:
        choice.action === "block" ? (
          <Ban aria-hidden className="size-3.5 shrink-0" />
        ) : (
          <ShieldCheck aria-hidden className="size-3.5 shrink-0" />
        ),
      danger: choice.action === "block",
      disabled: choice.enabled === false,
      onSelect: () => choose(choice.scope, choice.action),
    })),
    ...extraItems,
  ];

  return (
    <Menu
      open={menuOpen}
      onClose={() => setMenuOpen(false)}
      anchor={anchor}
      items={items}
      elevated={elevated}
      label={`Actions for ${subject}`}
    />
  );
}
