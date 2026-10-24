import type { Config } from "tailwindcss";

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--background) / <alpha-value>)",
        surface: "hsl(var(--surface) / <alpha-value>)",
        "surface-muted": "hsl(var(--surface-muted) / <alpha-value>)",
        foreground: "hsl(var(--foreground) / <alpha-value>)",
        "foreground-muted": "hsl(var(--foreground-muted) / <alpha-value>)",
        "foreground-subtle": "hsl(var(--foreground-subtle) / <alpha-value>)",
        border: "hsl(var(--border) / <alpha-value>)",
        "border-strong": "hsl(var(--border-strong) / <alpha-value>)",
        ring: "hsl(var(--ring) / <alpha-value>)",
        primary: "hsl(var(--primary) / <alpha-value>)",
        "primary-foreground": "hsl(var(--primary-foreground) / <alpha-value>)",
        secondary: "hsl(var(--secondary) / <alpha-value>)",
        "secondary-foreground": "hsl(var(--secondary-foreground) / <alpha-value>)",
        success: "hsl(var(--success) / <alpha-value>)",
        "success-foreground": "hsl(var(--success-foreground) / <alpha-value>)",
        warning: "hsl(var(--warning) / <alpha-value>)",
        "warning-foreground": "hsl(var(--warning-foreground) / <alpha-value>)",
        danger: "hsl(var(--danger) / <alpha-value>)",
        "danger-foreground": "hsl(var(--danger-foreground) / <alpha-value>)",
        info: "hsl(var(--info) / <alpha-value>)",
        accent: "hsl(var(--accent) / <alpha-value>)",
        // Protocol legend — one color per protocol everywhere (ui_rules.md §6).
        "proto-dns": "hsl(var(--proto-dns) / <alpha-value>)",
        "proto-http": "hsl(var(--proto-http) / <alpha-value>)",
        "proto-tls": "hsl(var(--proto-tls) / <alpha-value>)",
        "proto-quic": "hsl(var(--proto-quic) / <alpha-value>)",
        "proto-icmp": "hsl(var(--proto-icmp) / <alpha-value>)",
        "proto-other": "hsl(var(--proto-other) / <alpha-value>)",
        // Ink-on-tint / chart-mark pairs (ui_tokens.md §1.1): same hue, legible lightness.
        "primary-strong": "hsl(var(--primary-strong) / <alpha-value>)",
        "success-strong": "hsl(var(--success-strong) / <alpha-value>)",
        "warning-strong": "hsl(var(--warning-strong) / <alpha-value>)",
        "danger-strong": "hsl(var(--danger-strong) / <alpha-value>)",
        "proto-dns-strong": "hsl(var(--proto-dns-strong) / <alpha-value>)",
        "proto-http-strong": "hsl(var(--proto-http-strong) / <alpha-value>)",
        "proto-tls-strong": "hsl(var(--proto-tls-strong) / <alpha-value>)",
        "proto-quic-strong": "hsl(var(--proto-quic-strong) / <alpha-value>)",
        "proto-icmp-strong": "hsl(var(--proto-icmp-strong) / <alpha-value>)",
        "proto-other-strong": "hsl(var(--proto-other-strong) / <alpha-value>)",
      },
      borderRadius: { lg: "var(--radius)", md: "calc(var(--radius) - 2px)" },
      // A table's sticky header does not scroll, so it never loses width to the scrollbar
      // the way its body does — right-aligned columns drift apart by exactly the bar's
      // width. This is the body's `px-3` plus that width, applied to the header alone; the
      // body reserves the gutter unconditionally (`.scrollbar-gutter-stable`) so the two
      // agree whether or not the rows currently overflow.
      padding: { "table-gutter": "calc(0.75rem + var(--scrollbar-size))" },
      // 8% tints for selected and blocked table rows (ui_rules.md §5); the rest of the
      // scale is Tailwind's default.
      opacity: { 8: "0.08" },
      boxShadow: { sm: "var(--shadow-sm)", md: "var(--shadow-md)" },
      fontFamily: { sans: ["Geist", "Inter", "system-ui", "sans-serif"], mono: ["Geist Mono", "ui-monospace", "monospace"] },
      fontSize: { display: ["30px", "36px"], h1: ["20px", "28px"], h2: ["16px", "24px"], body: ["14px", "20px"], sm: ["13px", "18px"], xs: ["12px", "16px"] },
      // Row heights and z-index scale from ui_tokens.md §3/§6, plus the two panel sizes
      // ui_rules.md fixes: the inspector drawer (§7) and the scroll height every data
      // table shares (§5).
      height: { row: "36px", "row-compact": "28px", "table-body": "28rem" },
      maxHeight: { "table-body": "28rem" },
      width: { drawer: "30rem" },
      zIndex: { sticky: "10", dropdown: "20", drawer: "30", modal: "40", toast: "50" },
      transitionTimingFunction: { enter: "cubic-bezier(0.16, 1, 0.3, 1)" },
      transitionDuration: { fast: "120ms", base: "180ms", slow: "240ms" },
      // Enter animations from ui_tokens.md §5 — opacity/transform only. Every usage is
      // wrapped in `motion-safe:` so `prefers-reduced-motion` skips them entirely.
      keyframes: {
        in: {
          from: { opacity: "0", transform: "translateX(1rem)" },
          to: { opacity: "1", transform: "translateX(0)" },
        },
        "fade-in": { from: { opacity: "0" }, to: { opacity: "1" } },
        "pop-in": {
          from: { opacity: "0", transform: "translateY(0.5rem) scale(0.98)" },
          to: { opacity: "1", transform: "translateY(0) scale(1)" },
        },
        "toast-in": {
          from: { opacity: "0", transform: "translateY(0.75rem)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
      },
      animation: {
        in: "in 180ms cubic-bezier(0.16, 1, 0.3, 1)",
        "fade-in": "fade-in 120ms cubic-bezier(0.16, 1, 0.3, 1)",
        "pop-in": "pop-in 180ms cubic-bezier(0.16, 1, 0.3, 1)",
        "toast-in": "toast-in 180ms cubic-bezier(0.16, 1, 0.3, 1)",
      },
    },
  },
  plugins: [],
} satisfies Config;
