import { useCallback, useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { IconButton } from "@/components/ui/icon-button";
import { cn } from "@/lib/cn";
import { isTopOverlay, popOverlay, pushOverlay } from "@/lib/overlay-stack";

/** Anything that can hold focus inside the panel, for the Tab cycle. */
const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export type ModalProps = {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string | undefined;
  children: ReactNode;
  /** Actions row, rendered bottom-right (`ui_rules.md` §8). */
  footer?: ReactNode;
  /** Destructive confirmations set this so a stray Escape or scrim click cannot dismiss. */
  dismissible?: boolean;
  className?: string | undefined;
};

/** Centered dialog per `ui_rules.md` §8 — scrim, focus trap, Escape to close, focus
 *  returned to whatever opened it. Used for create/edit rule and destructive confirms. */
export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  dismissible = true,
  className,
}: ModalProps): JSX.Element | null {
  const panelRef = useRef<HTMLDivElement>(null);
  const opener = useRef<Element | null>(null);

  const trapFocus = useCallback((event: KeyboardEvent) => {
    const panel = panelRef.current;
    if (!panel) return;
    const targets = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
    const first = targets[0];
    const last = targets[targets.length - 1];
    if (!first || !last) {
      event.preventDefault();
      return;
    }
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    opener.current = document.activeElement;
    // Focus the first control rather than the panel, so typing starts in the form.
    const targets = panelRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE);
    (targets?.[0] ?? panelRef.current)?.focus();

    // Registered even when not dismissible: a non-dismissible confirm must still swallow
    // Escape rather than letting it close the drawer or view underneath.
    const layer = pushOverlay();
    const onKeyDown = (event: KeyboardEvent): void => {
      if (!isTopOverlay(layer)) return;
      if (event.key === "Escape" && dismissible) onClose();
      if (event.key === "Tab") trapFocus(event);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      popOverlay(layer);
      window.removeEventListener("keydown", onKeyDown);
      if (opener.current instanceof HTMLElement) opener.current.focus();
    };
  }, [open, onClose, dismissible, trapFocus]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-modal flex items-center justify-center p-6">
      {/* Scrim colour is fixed by ui_rules.md §8 rather than themed. */}
      <div
        aria-hidden
        onClick={() => dismissible && onClose()}
        className="absolute inset-0 bg-black/50 motion-safe:animate-fade-in"
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={cn(
          "relative flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-lg border border-border bg-surface shadow-md",
          "focus-visible:outline-none motion-safe:animate-pop-in",
          className,
        )}
      >
        <header className="flex items-start justify-between gap-4 border-b border-border px-4 py-3">
          <div>
            <h2 className="text-h2 font-semibold tracking-tight">{title}</h2>
            {description ? (
              <p className="mt-1 text-xs text-foreground-muted">{description}</p>
            ) : null}
          </div>
          {dismissible ? (
            <IconButton
              icon={<X aria-hidden className="size-4" />}
              label={`Close ${title.toLowerCase()}`}
              onClick={onClose}
            />
          ) : null}
        </header>
        <div className="flex-1 overflow-auto px-4 py-4">{children}</div>
        {footer ? (
          <footer className="flex items-center justify-end gap-2 border-t border-border px-4 py-3">
            {footer}
          </footer>
        ) : null}
      </div>
    </div>
  );
}
