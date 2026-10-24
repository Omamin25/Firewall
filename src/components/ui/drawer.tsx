import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/cn";
import { isTopOverlay, popOverlay, pushOverlay } from "@/lib/overlay-stack";

export type DrawerProps = {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
};

/** Right-side drawer (z-30). Closes on Escape and returns focus to the opener. */
export function Drawer({ open, onClose, title, children, footer }: DrawerProps): JSX.Element | null {
  const panelRef = useRef<HTMLDivElement>(null);
  const opener = useRef<Element | null>(null);

  useEffect(() => {
    if (!open) return;
    opener.current = document.activeElement;
    panelRef.current?.focus();
    // A dialog or menu opened from inside the drawer takes Escape first.
    const layer = pushOverlay();
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape" && isTopOverlay(layer)) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      popOverlay(layer);
      window.removeEventListener("keydown", onKeyDown);
      if (opener.current instanceof HTMLElement) opener.current.focus();
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      ref={panelRef}
      role="dialog"
      aria-modal="false"
      aria-label={title}
      tabIndex={-1}
      className={cn(
        "fixed right-0 top-0 z-drawer flex h-full w-drawer max-w-full flex-col border-l border-border bg-surface shadow-md",
        "motion-safe:animate-in focus-visible:outline-none",
      )}
    >
      <header className="flex items-center justify-between border-b border-border px-4 py-3">
        <h2 className="text-h2 font-semibold tracking-tight">{title}</h2>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close inspector"
          className="rounded-md p-1 text-foreground-muted hover:bg-surface-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <X aria-hidden className="size-4" />
        </button>
      </header>
      <div className="flex-1 overflow-auto px-4 py-3">{children}</div>
      {footer ? <footer className="border-t border-border px-4 py-3">{footer}</footer> : null}
    </div>
  );
}
