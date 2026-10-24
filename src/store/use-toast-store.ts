import { create } from "zustand";

export type ToastVariant = "success" | "danger";

export type Toast = { id: number; variant: ToastVariant; message: string };

export type ToastState = {
  toasts: Toast[];
  /** Returns the id so a caller can dismiss early if it wants to. */
  push: (variant: ToastVariant, message: string) => number;
  dismiss: (id: number) => void;
};

/** Newest toasts render at the bottom of the stack; the list is capped so a burst of
 *  rule changes cannot cover the app. */
const MAX_TOASTS = 4;

let nextId = 1;

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  push: (variant, message) => {
    const id = nextId++;
    set((state) => ({ toasts: [...state.toasts, { id, variant, message }].slice(-MAX_TOASTS) }));
    return id;
  },
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));

/** Convenience wrappers so callers do not repeat the variant string. */
export function toastSuccess(message: string): void {
  useToastStore.getState().push("success", message);
}

export function toastError(message: string): void {
  useToastStore.getState().push("danger", message);
}
