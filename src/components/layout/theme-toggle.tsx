import { Moon, Sun } from "lucide-react";

export type ThemeToggleProps = { dark: boolean; onToggle: () => void };
export function ThemeToggle({ dark, onToggle }: ThemeToggleProps): JSX.Element {
  return <button type="button" onClick={onToggle} className="flex size-9 items-center justify-center rounded-md text-foreground-muted hover:bg-surface-muted" aria-label="Toggle color theme">{dark ? <Sun aria-hidden size={16} /> : <Moon aria-hidden size={16} />}</button>;
}
