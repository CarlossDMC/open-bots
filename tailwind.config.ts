import type { Config } from "tailwindcss";
import { cubicBezier, durations, easings } from "./src/styles/motion-tokens";

// Every visual value resolves to a CSS custom property declared in src/styles/tokens.css.
const color = (token: string) => `hsl(var(--${token}) / <alpha-value>)`;
const ms = (value: number) => `${value}ms`;

export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: color("background"),
        surface: color("surface"),
        card: color("card"),
        muted: color("muted"),
        accent: color("accent"),
        overlay: "hsl(var(--overlay) / var(--overlay-opacity))",
        ring: color("ring"),
        selection: color("selection"),
        border: {
          DEFAULT: color("border"),
          subtle: color("border-subtle"),
          strong: color("border-strong")
        },
        foreground: {
          DEFAULT: color("foreground"),
          secondary: color("foreground-secondary"),
          muted: color("foreground-muted"),
          subtle: color("foreground-subtle"),
          faint: color("foreground-faint")
        },
        primary: { DEFAULT: color("primary"), foreground: color("primary-foreground") },
        status: {
          running: color("status-running"),
          waiting: color("status-waiting"),
          queued: color("status-queued"),
          success: color("status-success"),
          danger: color("status-danger"),
          neutral: color("status-neutral")
        },
        danger: {
          DEFAULT: color("danger"),
          foreground: color("danger-foreground"),
          muted: color("danger-muted"),
          border: color("danger-border")
        },
        warning: {
          DEFAULT: color("warning"),
          foreground: color("warning-foreground"),
          muted: color("warning-muted"),
          border: color("warning-border")
        },
        identity: {
          indigo: color("identity-indigo"),
          cyan: color("identity-cyan"),
          emerald: color("identity-emerald"),
          amber: color("identity-amber"),
          rose: color("identity-rose"),
          violet: color("identity-violet")
        },
        "avatar-eye": color("avatar-eye")
      },
      fontFamily: { sans: ["var(--font-sans)"], mono: ["var(--font-mono)"] },
      fontSize: {
        "3xs": "var(--text-3xs)",
        "2xs": "var(--text-2xs)",
        "xs-plus": "var(--text-xs-plus)"
      },
      borderRadius: {
        DEFAULT: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
        xl: "var(--radius-xl)"
      },
      boxShadow: { panel: "var(--shadow-panel)" },
      transitionDuration: {
        fast: ms(durations.fast),
        base: ms(durations.base),
        slow: ms(durations.slow)
      },
      transitionTimingFunction: {
        standard: cubicBezier(easings.standard),
        exit: cubicBezier(easings.exit)
      },
      keyframes: {
        "fade-in": {
          from: { opacity: "0", transform: "translateY(4px)" },
          to: { opacity: "1", transform: "translateY(0)" }
        }
      },
      animation: { "fade-in": `fade-in ${ms(durations.base)} ${cubicBezier(easings.standard)}` }
    }
  },
  plugins: []
} satisfies Config;
