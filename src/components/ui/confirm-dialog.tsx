import { Button } from "@/components/ui/button";
import { Modal } from "@/components/ui/modal";

export type ConfirmDialogProps = {
  open: boolean;
  title: string;
  /** Plain-language consequence of confirming. */
  detail: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  busy?: boolean;
};

/** Destructive confirmation (`ui_rules.md` §3, §8). Not dismissible by scrim or Escape:
 *  the choice has to be made deliberately, so Cancel is the only way out. */
export function ConfirmDialog({
  open,
  title,
  detail,
  confirmLabel,
  onConfirm,
  onCancel,
  busy = false,
}: ConfirmDialogProps): JSX.Element | null {
  return (
    <Modal
      open={open}
      onClose={onCancel}
      title={title}
      dismissible={false}
      className="max-w-md"
      footer={
        <>
          <Button variant="outline" size="sm" onClick={onCancel} disabled={busy}>
            Cancel
          </Button>
          <Button variant="destructive" size="sm" onClick={onConfirm} disabled={busy}>
            {busy ? "Working…" : confirmLabel}
          </Button>
        </>
      }
    >
      <p className="text-sm text-foreground-muted">{detail}</p>
    </Modal>
  );
}
