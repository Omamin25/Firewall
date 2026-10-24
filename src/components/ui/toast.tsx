import { useEffect } from "react";
import { CircleCheck, TriangleAlert, X } from "lucide-react";
import { IconButton } from "@/components/ui/icon-button";
import { cn } from "@/lib/cn";
import { useToastStore, type Toast as ToastData } from "@/store/use-toast-store";

/** Transient confirmations only (`ui_rules.md` §9) — anything needing action is an
 *  ErrorState, not a toast. */
const DISMISS_AFTER_MS = 4_000;

/** Bottom-right toast stack (z-toast). Mount once, near the app root. */
export function Toaster(): JSX.Element | null {
  const toasts = useToastStore((state) => state.toasts);
  if (toasts.length === 0) return null;
  return (
    <div
      aria-live="polite"
      aria-atomic="false"
      className="pointer-events-none fixed bottom-6 right-6 z-toast flex w-80 flex-col gap-2"
    >
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} />
      ))}
    </div>
  );
}

function Toast({ toast }: { toast: ToastData }): JSX.Element {
  const dismiss = useToastStore((state) => state.dismiss);

  useEffect(() => {
    const timer = window.setTimeout(() => dismiss(toast.id), DISMISS_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, [toast.id, dismiss]);

  const danger = toast.variant === "danger";
  return (
    <div
      role="status"
      className={cn(
        "pointer-events-auto flex items-start gap-2 rounded-md border bg-surface p-3 shadow-md",
        "motion-safe:animate-toast-in",
        danger ? "border-danger" : "border-border",
      )}
    >
      {/* Icon plus text, never colour alone (`ui_rules.md` §10). */}
      {danger ? (
        <TriangleAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-danger-strong" />
      ) : (
        <CircleCheck aria-hidden className="mt-0.5 size-4 shrink-0 text-success-strong" />
      )}
      <p className="min-w-0 flex-1 text-sm">{toast.message}</p>
      <IconButton
        icon={<X aria-hidden className="size-3.5" />}
        label="Dismiss notification"
        onClick={() => dismiss(toast.id)}
      />
    </div>
  );
}
