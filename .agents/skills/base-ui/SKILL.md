---
name: base-ui
description: Reference for using @base-ui/react unstyled components within the Vulcanum frontend. Covers import conventions, Preact compat, Tailwind v4 + OKLCH design token styling, compound component wrappers, and state management rules. Use it when in need of designing or creating a new component for the app or refactoring existing ones
---

# Base UI (Vulcanum Component Library Skill)

Vulcanum uses **@base-ui/react@1.5.0** as its unstyled component foundation. The frontend is **Preact 10** (via `@preact/preset-vite` + `preact/compat` aliases for React compatibility) and **Tailwind CSS v4** with **OKLCH** design tokens.

This skill is the single reference for creating or modifying shared UI components.

## 1. Folder Organization & Import Rules

The codebase is split into **two distinct zones**:

| Zone                         | Path                        | Responsibility                                                                                                                                                      | Import rule                                                                               |
| ---------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| **Shared UI primitives**     | `src/components/shared/ui/` | The **only** place that imports from `@base-ui/react`. Wraps every unstyled primitive with Vulcanum design tokens, focus rings, sizing, and accessibility defaults. | ✅ `@base-ui/react/*` allowed here ONLY.                                                  |
| **Feature / app components** | `src/components/<feature>/` | Composes **shared UI primitives** into domain-specific views.                                                                                                       | ❌ Never import `@base-ui/react/*` directly. Always consume from `components/shared/ui/`. |

### Enforcement

- If a feature needs a new primitive that does not yet have a shared wrapper, **create the wrapper in `shared/ui/` first**, then use it in the feature. Do not import the raw Base UI component inline inside a feature file.
- If an existing feature file imports `@base-ui/react/...`, treat it as a bug and move the import into a new or existing `shared/ui/` wrapper.
- Shared wrappers may be simple (e.g. `Button.view.tsx`) or compound (e.g. `Dialog.view.tsx`). Either way, the wrapper is the single owner of the `@base-ui/react` dependency.

### Why

This guarantees that design tokens, focus styles, accessibility patterns, and sizing are consistent across every screen. If a designer changes the button hover color, only `shared/ui/Button.view.tsx` needs updating — not every feature file.

## 2. Tech Context

- **Renderer**: Preact 10 (`jsxImportSource: "preact"` in `tsconfig.json` and Vite config).
- **React compat**: `react` and `react-dom` are aliased to `preact/compat` in both `vite.config.ts` and `tsconfig.json` paths.
- **State**:
  - Server state → `@tanstack/react-query` via `useApiQuery` / `useApiMutation`.
  - Global/shared client state → `@preact/signals` (`signal()` in stores).
  - Component-local / ephemeral UI state → `useSignal` or Base UI’s built-in state hooks (`useDialogRoot`, etc.).
- **Styling**: Tailwind CSS v4 only; no CSS-in-JS.
- **Class prop**: In Preact JSX, always use `class="..."`, **not** `className`. The compat layer generally handles `className`, but native Preact behavior uses `class`. Avoid mixing both.

## 3. Import Conventions

**Only files inside `src/components/shared/ui/` are permitted to import from `@base-ui/react`.**

All other code — feature views, containers, pages, hooks — must import the styled wrapper from the shared UI folder.

```tsx
// ✅ SHARED UI (allowed to import Base UI directly)
// In components/shared/ui/Button.view.tsx
import { Button as BaseButton } from "@base-ui/react/button";

// ❌ FEATURE (never do this)
// In components/projects/ui/ProjectCard.view.tsx
import { Button } from "@base-ui/react/button"; // BANNED

// ✅ FEATURE (correct)
// In components/projects/ui/ProjectCard.view.tsx
import { Button } from "../../shared/ui/Button.view";
```

Base UI components are imported from `@base-ui/react/<component>` exactly as documented in the Base UI docs. Because of the `preact/compat` alias, React-specific hooks (`useState`, `useEffect`, `useId`) and JSX runtime are automatically satisfied by Preact. No extra shims are needed.

```tsx
// Allowed inside shared/ui/ only: default imports of the full API
import * as Dialog from "@base-ui/react/dialog";

// Allowed inside shared/ui/ only: named imports when tree-shaking is preferred
import { Button } from "@base-ui/react/button";
import { Input } from "@base-ui/react/input";
```

**Preact-specific rules**:

- Do **not** import from `"react"` directly; if you need hooks, import from `"preact/compat"` or `"preact/hooks"`.
- In JSX, write `class="..."`.
- Base UI compound components (`Dialog.Root`, `Dialog.Trigger`, etc.) work normally through the compat layer.

## 4. Design Tokens → Tailwind Utilities

Tokens are defined in `src/style.css` as `@theme` mappings to CSS custom properties. Use the Tailwind utility names below; never hard-code raw OKLCH values in components.

| Semantic usage              | Tailwind utility                       | Maps to token (dark)         |
| --------------------------- | -------------------------------------- | ---------------------------- |
| Page background             | `bg-bg-page`                           | `oklch(0% 0 0)`              |
| Card / panel background     | `bg-bg-card`                           | `oklch(14% 0 0)`             |
| Input background            | `bg-bg-input`                          | `oklch(20% 0 0)`             |
| Hover background            | `bg-bg-hover`                          | `oklch(22% 0 0)`             |
| Active / pressed background | `bg-bg-active`                         | `oklch(23% 0 0)`             |
| Primary text                | `text-text-primary`                    | `oklch(100% 0 0)`            |
| Secondary text              | `text-text-secondary`                  | `oklch(100% 0 0 / 0.8)`      |
| Muted text                  | `text-text-muted`                      | `oklch(65% 0 0)`             |
| Default border              | `border-border-base`                   | `oklch(26% 0 0)`             |
| Focus border                | `border-border-focus`                  | `oklch(65% 0.2 30)`          |
| Accent (CTA)                | `text-accent`, `bg-accent`             | `oklch(65% 0.22 30)`         |
| Accent light                | `text-accent-light`, `bg-accent-light` | `oklch(75% 0.15 40)`         |
| Error text                  | `text-error`                           | `oklch(55% 0.2 25)`          |
| Error background            | `bg-error-bg`                          | `oklch(55% 0.2 25 / 0.15)`   |
| Error border                | `border-error-border`                  | `oklch(55% 0.2 25 / 0.3)`    |
| Success text                | `text-success`                         | `oklch(65% 0.16 155)`        |
| Success background          | `bg-success-bg`                        | `oklch(65% 0.16 155 / 0.15)` |
| Warning text                | `text-warning`                         | `oklch(75% 0.15 85)`         |
| Warning background          | `bg-warning-bg`                        | `oklch(75% 0.15 85 / 0.15)`  |

Light theme tokens are available automatically when `html[data-theme="light"]` is active; Tailwind dark-mode is handled via the `data-theme` attribute.

## 5. Wrapper Component Pattern

Every unstyled Base UI component used in Vulcanum **must** be wrapped in a styled view inside `src/components/shared/ui/ComponentName.view.tsx`. This guarantees a single import for feature code, a single place for design tokens, and consistent overrides.

### File naming

- `src/components/shared/ui/Button.view.tsx`
- `src/components/shared/ui/Input.view.tsx`
- `src/components/shared/ui/Dialog.view.tsx`

### Simple wrapper (single-part component)

Use `forwardRef` from `preact/compat` so the wrapper remains transparent to `ref` consumers:

```tsx
import { forwardRef } from "preact/compat";
import { Button as BaseButton } from "@base-ui/react/button";
import type { ComponentPropsWithoutRef } from "preact/compat";
import type { JSX } from "preact";
import { clsx } from "clsx";

interface ButtonProps extends ComponentPropsWithoutRef<"button"> {
  variant?: "primary" | "secondary" | "ghost";
}

const VARIANT_MAP: Record<string, string> = {
  primary:
    "bg-text-primary text-bg-page px-4 py-3 text-sm uppercase tracking-wide hover:opacity-90",
  secondary:
    "border border-border-base text-text-primary px-4 py-3 text-sm uppercase tracking-wide hover:bg-bg-hover",
  ghost:
    "text-text-secondary hover:text-text-primary hover:bg-bg-hover px-3 py-2 text-sm",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", class: classProp, ...rest }, ref): JSX.Element => (
    <BaseButton
      ref={ref}
      class={clsx(
        "inline-flex items-center justify-center cursor-pointer transition-colors duration-fast",
        VARIANT_MAP[variant],
        classProp,
      )}
      {...rest}
    />
  ),
);
```

**Rules for simple wrappers**:

- Always pass through the `class` prop using `clsx` so callers can override / extend.
- Combine the design token classes (the static look) with the caller-supplied class.
- Keep wrappers thin: only styling, layout, and prop mapping. No logic.

### Compound wrapper (multi-part component)

For compound APIs (Dialog, Toast, Select, etc.), export a single object with sub-components so features consume one import:

```tsx
import * as BaseDialog from "@base-ui/react/dialog";
import type { JSX } from "preact";
import { clsx } from "clsx";

// --- Dialog Root ---

interface DialogProps {
  children: JSX.Element | JSX.Element[];
  defaultOpen?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export const Dialog = ({ children, ...props }: DialogProps): JSX.Element => (
  <BaseDialog.Root {...props}>{children}</BaseDialog.Root>
);

// --- Dialog Trigger ---

interface DialogTriggerProps {
  children: JSX.Element;
  class?: string;
}

Dialog.Trigger = ({
  children,
  class: classProp,
}: DialogTriggerProps): JSX.Element => (
  <BaseDialog.Trigger
    class={clsx(
      "cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-border-focus",
      classProp,
    )}
  >
    {children}
  </BaseDialog.Trigger>
);

// --- Dialog Backdrop ---

interface DialogBackdropProps {
  class?: string;
}

Dialog.Backdrop = ({ class: classProp }: DialogBackdropProps): JSX.Element => (
  <BaseDialog.Backdrop
    class={clsx(
      "fixed inset-0 bg-bg-page/80 backdrop-blur-sm transition-opacity",
      classProp,
    )}
  />
);

// --- Dialog Popup ---

interface DialogPopupProps {
  children: JSX.Element | JSX.Element[];
  class?: string;
}

Dialog.Popup = ({
  children,
  class: classProp,
}: DialogPopupProps): JSX.Element => (
  <BaseDialog.Popup
    class={clsx(
      "fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2",
      "bg-bg-card border border-border-base p-5 shadow-modal max-w-lg w-full",
      "focus:outline-none",
      classProp,
    )}
  >
    {children}
  </BaseDialog.Popup>
);

// --- Dialog Title ---

interface DialogTitleProps {
  children: string;
  class?: string;
}

Dialog.Title = ({
  children,
  class: classProp,
}: DialogTitleProps): JSX.Element => (
  <BaseDialog.Title
    class={clsx("text-text-primary text-lg font-medium mb-3", classProp)}
  >
    {children}
  </BaseDialog.Title>
);

// --- Dialog Description ---

interface DialogDescriptionProps {
  children: string;
  class?: string;
}

Dialog.Description = ({
  children,
  class: classProp,
}: DialogDescriptionProps): JSX.Element => (
  <BaseDialog.Description
    class={clsx("text-text-secondary text-sm mb-4", classProp)}
  >
    {children}
  </BaseDialog.Description>
);

// --- Dialog Close ---

interface DialogCloseProps {
  children: JSX.Element;
  class?: string;
}

Dialog.Close = ({
  children,
  class: classProp,
}: DialogCloseProps): JSX.Element => (
  <BaseDialog.Close
    class={clsx(
      "cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-border-focus",
      classProp,
    )}
  >
    {children}
  </BaseDialog.Close>
);
```

Usage in a feature:

```tsx
import { Dialog } from "../../components/shared/ui/Dialog.view";
import { Button } from "../../components/shared/ui/Button.view";

export const DeleteConfirm = (): JSX.Element => (
  <Dialog>
    <Dialog.Trigger>
      <Button variant="ghost">Delete</Button>
    </Dialog.Trigger>
    <Dialog.Backdrop />
    <Dialog.Popup>
      <Dialog.Title>Confirm deletion</Dialog.Title>
      <Dialog.Description>This cannot be undone.</Dialog.Description>
      <div class="flex gap-3 justify-end mt-5">
        <Dialog.Close>
          <Button variant="secondary">Cancel</Button>
        </Dialog.Close>
        <Button variant="primary">Delete</Button>
      </div>
    </Dialog.Popup>
  </Dialog>
);
```

## 7. Component Availability Table

All Base UI components below may be wrapped in `components/shared/ui/` when a feature needs them. Do not import Base UI directly in feature code. Always consume through a wrapper.

| Base UI Component | Vulcanum Wrapper Path                       | Notes                                                                  |
| ----------------- | ------------------------------------------- | ---------------------------------------------------------------------- |
| `Button`          | `components/shared/ui/Button.view.tsx`      | Primary CTA, secondary, Ghost                                          |
| `Input`           | `components/shared/ui/Input.view.tsx`       | Text inputs with project focus/error states                            |
| `Dialog`          | `components/shared/ui/Dialog.view.tsx`      | Compound wrapper (Trigger, Backdrop, Popup, Title, Description, Close) |
| `Toast`           | `components/shared/ui/Toast.view.tsx`       | Compound wrapper (Provider, Toast)                                     |
| `Select`          | `components/shared/ui/Select.view.tsx`      | Compound wrapper (Trigger, Value, Popup, Option)                       |
| `Switch`          | `components/shared/ui/Switch.view.tsx`      | Toggle with project track/thumb tokens                                 |
| `Checkbox`        | `components/shared/ui/Checkbox.view.tsx`    | Check indicator with project tokens                                    |
| `Tooltip`         | `components/shared/ui/Tooltip.view.tsx`     | Compound wrapper (Provider, Trigger, Positioner)                       |
| `Popover`         | `components/shared/ui/Popover.view.tsx`     | Compound wrapper (Trigger, Positioner, Popup, Arrow)                   |
| `Menu`            | `components/shared/ui/Menu.view.tsx`        | Compound wrapper (Trigger, Positioner, Popup, Item)                    |
| `Collapsible`     | `components/shared/ui/Collapsible.view.tsx` | Expand/collapse sections                                               |
| `Tabs`            | `components/shared/ui/Tabs.view.tsx`        | Compound wrapper (List, Tab, Panel)                                    |
| `Field`           | `components/shared/ui/Field.view.tsx`       | Wrapper for label + error text pairing                                 |
| `Form`            | `components/shared/ui/Form.view.tsx`        | Validation integration wrapper                                         |
| `Separator`       | `components/shared/ui/Separator.view.tsx`   | Minimal divider line                                                   |

If a needed component is not yet wrapped, create it in `components/shared/ui/` following the patterns above. Keep wrapper files under 150 lines; split sub-parts into separate files if the compound API is large.

## 8. State Management Rules

| State type                                      | Tool                                            | When to use                            |
| ----------------------------------------------- | ----------------------------------------------- | -------------------------------------- |
| **Open / closed** (Dialog, Popover Toast, Menu) | Base UI `open` / `defaultOpen` + `onOpenChange` | Local ephemeral UI state               |
| **Form field values**                           | Base UI `Field` / `Form` or `useSignal`         | Local, unless shared across components |
| **Shared selection / filter**                   | `@preact/signals` store                         | Cross-component or persisted           |
| **Server data**                                 | `@tanstack/react-query`                         | Never duplicate into signals           |
| **Theme / auth token**                          | `@preact/signals` global store                  | Singletons                             |

Guidelines:

- Use Base UI’s built-in state APIs (`Dialog.Root open={...} onOpenChange={...}`) for open/closed, focus, and selection state that lives inside the component tree.
- Use `@preact/signals` only when state is shared between distant components or must survive unmounting.
- Never store server-fetched data in signals if React Query already caches it.

## 9. Styling Details

- **No border-radius**: unless the Base UI primitive explicitly requires it for focus rings, use `rounded-none` (0px) by default per the design system.
- **Borders**: use `border border-border-base` for cards/panels; `border-white/10` is acceptable for very faint dividers.
- **Hover**: `hover:bg-bg-hover` or `hover:border-border-focus`; do not invent new colors.
- **Focus rings**: `focus-visible:ring-2 focus-visible:ring-border-focus` is the standard focus indicator.
- **Transitions**: prefer `transition-colors duration-fast` (maps to 150ms) for interactive elements.
- **Flexbox gap**: always use `flex` with `gap-*`; never use `space-y-*` or margins on children for spacing.

## 10. Examples

### Button

See `components/shared/ui/Button.view.tsx` in the wrapper pattern above.

### Input

```tsx
import { forwardRef } from "preact/compat";
import { Input as BaseInput } from "@base-ui/react/input";
import type { ComponentPropsWithoutRef } from "preact/compat";
import type { JSX } from "preact";
import { clsx } from "clsx";

interface InputProps extends ComponentPropsWithoutRef<"input"> {
  invalid?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ invalid, class: classProp, ...rest }, ref): JSX.Element => (
    <BaseInput
      ref={ref}
      class={clsx(
        "w-full bg-bg-input border px-3 py-2 text-sm text-text-primary outline-none transition-colors duration-fast",
        "placeholder:text-text-muted focus:border-border-focus",
        invalid ? "border-error" : "border-border-base",
        classProp,
      )}
      {...rest}
    />
  ),
);
```

### Dialog

See the compound wrapper example in section 5.

## 11. Migration Policy

- **Do not** migrate existing feature components to Base UI wrappers unless explicitly tasked.
- New shared UI components **must** be built as Base UI wrappers.
- Existing one-off components can remain as-is until a refactor story is created.

## 12. Further Reading

- Base UI docs: <https://base-ui.com/react>
- Preact signals: <https://preactjs.com/guide/v10/signals/>
- Tailwind CSS v4: <https://tailwindcss.com/docs/styling-with-utility-classes>
