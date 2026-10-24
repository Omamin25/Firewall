import { useEffect, useRef, type ReactNode } from "react";
import { cn } from "@/lib/cn";
import { isTopOverlay, popOverlay, pushOverlay } from "@/lib/overlay-stack";

export type MenuEntry =
  | { kind: "label"; label: string }
  | {
      kind: "action";
      label: string;
      icon?: ReactNode;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => void;
    };

export type MenuProps = {
  open: boolean;
  onClose: () => void;
  /** Viewport coordinates of the top-left corner — a trigger rect or a right-click point. */
  anchor: { x: number; y: number };
  items: readonly MenuEntry[];
  /** Accessible name for the menu as a whole. */
  label: string;
  /** Set when the menu is opened from inside a drawer or modal, which sit above the
   *  dropdown layer in the z-scale (`ui_tokens.md` §6). */
  elevated?: boolean;
};

/** Estimated panel size, used only to keep the menu inside the window. */
const ESTIMATED_WIDTH = 240;
const ITEM_HEIGHT = 32;

/** Small popover menu (z-dropdown) shared by row hover actions and right-click context
 *  menus (`ui_rules.md` §5). Arrow keys move, Enter activates, Escape closes, and a click
 *  anywhere else dismisses it. */
export function Menu({
  open,
  onClose,
  anchor,
  items,
  label,
  elevated = false,
}: MenuProps): JSX.Element | null {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const actions = (): HTMLElement[] => [
      ...(panelRef.current?.querySelectorAll<HTMLElement>("[data-menu-action]") ?? []),
    ];
    actions()[0]?.focus();
    const layer = pushOverlay();

    const onKeyDown = (event: KeyboardEvent): void => {
      if (!isTopOverlay(layer)) return;
      if (event.key === "Escape") {
        onClose();
        return;
      }
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      const targets = actions();
      if (targets.length === 0) return;
      const current = targets.findIndex((item) => item === document.activeElement);
      const step = event.key === "ArrowDown" ? 1 : -1;
      const next = (current + step + targets.length) % targets.length;
      targets[next]?.focus();
    };

    const onPointerDown = (event: PointerEvent): void => {
      if (!(event.target instanceof Node)) return;
      if (!panelRef.current?.contains(event.target)) onClose();
    };

    window.addEventListener("keydown", onKeyDown);
    // Capture phase: the menu closes before the click reaches whatever is underneath.
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      popOverlay(layer);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [open, onClose]);

  if (!open) return null;

  const height = items.length * ITEM_HEIGHT;
  const left = Math.max(0, Math.min(anchor.x, window.innerWidth - ESTIMATED_WIDTH));
  const top = Math.max(0, Math.min(anchor.y, Math.max(0, window.innerHeight - height)));

  return (
    <div
      ref={panelRef}
      role="menu"
      aria-label={label}
      // Position is per-instance geometry, not a themable value.
      style={{ left, top }}
      className={cn(
        "fixed min-w-56 overflow-hidden rounded-lg border border-border bg-surface py-1 shadow-md",
        "motion-safe:animate-pop-in",
        elevated ? "z-modal" : "z-dropdown",
      )}
    >
      {items.map((item, index) =>
        item.kind === "label" ? (
          <p
            key={`${item.label}-${index}`}
            className="px-3 pb-1 pt-2 text-xs font-medium uppercase tracking-wide text-foreground-subtle"
          >
            {item.label}
          </p>
        ) : (
          <button
            key={`${item.label}-${index}`}
            type="button"
            role="menuitem"
            data-menu-action
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
            className={cn(
              "flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm",
              "focus-visible:outline-none focus-visible:bg-surface-muted hover:bg-surface-muted",
              "disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent",
              item.danger ? "text-danger-strong" : "text-foreground",
            )}
          >
            {item.icon}
            <span className="truncate">{item.label}</span>
          </button>
        ),
      )}
    </div>
  );
}
