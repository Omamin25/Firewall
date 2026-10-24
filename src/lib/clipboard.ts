import { toastError, toastSuccess } from "@/store/use-toast-store";

/** Copies a value and reports the outcome as a toast. Uses the webview clipboard rather
 *  than a Tauri plugin: it needs no extra capability, and copying is a UI concern with no
 *  privileged part to it. */
export async function copyToClipboard(value: string, label: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(value);
    toastSuccess(`${label} copied.`);
  } catch {
    toastError(`${label} could not be copied to the clipboard.`);
  }
}
