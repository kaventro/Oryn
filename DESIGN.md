# DESIGN.md — Oryn desktop UI (vanilla TypeScript + Vite + Tauri)

Oryn is a keyboard-driven dual-pane file manager. There is no React, Vue, or Tailwind. The window is `src/index.html`, controllers live in `src/modules/`, tokens in `src/styles/01-vars.css`, and sheets are bundled through `src/styles.css`. Native commands live in `desktop/`. Do not add a UI framework to use a component kit.

## 1. Tokens and surfaces (use what exists)

New colour, radius, and type go in `src/styles/01-vars.css`. Prefer `var(--…)` over hex. `CONTRIBUTING.md` already requires this.

**Surfaces** (dark-only today): `--mac-bg` / `--nx-bg` `#141416`, `--mac-card-bg` / `--nx-bg-elevated` `#242428`, `--mac-sidebar-bg`, `--mac-divider`, `--mac-card-border`, `--mac-glass-bg`, `--mac-glass-border`. Semantic aliases: `--background`, `--surface`, `--on-surface` `#f5f5f7`, `--on-surface-variant` `#8e8e93`, `--error` `#ff453a`. The window is a dark graphite workspace; panes sit on the same tone as the sidebar, separated by the hairline `--mac-divider`.

**Accent:** one at a time. Default Graphite Blue is `--primary` / `--mac-blue` `#0a84ff`. User trays are `html[data-tray]` via `AVAILABLE_THEMES` in `src/modules/settings/settingsStorage.ts`: `ocean`, `ember`, `emerald`, `purple`, `mono`. Semantic colours (`--mac-green`, `--mac-red`, `--mac-orange`, `--mac-yellow`, `--tag-*`) mean one thing (Git, status, tags). Do not invent a second chrome accent. Prefer `var(--primary)` over hardcoded `#0a84ff` so tray themes still apply.

**Type:** `--font-display` and `--font-body` are the system stack (`-apple-system`, `"SF Pro Text"`, `"Helvetica Neue"`, sans-serif). `--font-mono` is `"SF Mono", Monaco, Menlo, Consolas` (paths, diffs, hashes, xterm). Do not load web fonts in the app window. File-list body is 13px (`.row-name` in `src/styles/04-rows.css`); chrome is 10–15px. This is a dense desktop app, not a marketing site. There is no type-scale token — match nearby controls instead of introducing 16px body text.

**Radius / shadow:** `--radius-xs` 4px through `--radius-xl` 18px, plus `--radius-pill`. Prefer `--radius-sm` (6px) for rows/controls and `--radius-lg` (14px) for dialogs. Shadows: `--shadow-sm`, `--shadow-md`, `--shadow-lg`, `--shadow-accent`.

**Spacing:** there is no `--space-*` scale. Stick to 4/8 px steps already in the layout (4, 8, 12, 16, 20, 24). Default row height is 30px (`.row` in `04-rows.css`). `--space-unit` is referenced in `src/styles/components/hotkeys.css` but not defined — do not add more undefined variables. `--font-sans` is used in `menu-command.css` / `overlays.css` but is not defined; use `--font-body`.

**Icons:** one in-repo SVG set. File-type icons: `src/modules/icons/iconDefinitions.ts` + `iconRegistry.ts`. Menu icons: `src/modules/menu/menuIcons.ts`. 24×24 viewBox, 16×16 render, stroke ~2, `currentColor` unless `.mac-icon--*` already sets a type colour. No emoji as icons. No Lucide / Heroicons / Font Awesome packages.

**Motion:** 100–250 ms on transform/opacity (existing: 0.10s f-keys, 0.15s buttons, 0.16s modal fade/scale, 0.22s sidebar). Honor `@media (prefers-reduced-motion: reduce)` for non-essential animation (today only `.sidebar-brand` in `src/styles/layout/top.css` does). There are no `--duration-*` tokens.

**Keyboard:** this app is keyboard-first. Shortcuts live in `docs/SHORTCUTS.md` and `src/modules/keyboardController.ts`. Keep focus visible, Tab order sane, and menus reachable from `src/modules/menu/`. Do not extend the global `html, body { outline: none !important }` in `01-vars.css`.

**Chrome map:** sidebar + dual/single panes (`src/styles/layout/`), list/grid/columns (`viewController.ts`, `columns/`), F-key bar and terminal drawer (`layout/terminal.css`, `@xterm/xterm`), dialogs (`styles/modals/`). On macOS, `body.platform-mac` reserves traffic-light space — do not invent a second title-bar.

**How to add UI:** new dialog markup goes in `src/index.html` with styles in `src/styles/modals/`; new controller in `src/modules/` wired from `src/app.ts`. New file-type icons get an `IconKey` in `iconDefinitions.ts` and a mapping in `iconRegistry.ts`. New menu actions get an SVG in `menuIcons.ts`. Do not introduce CSS-in-JS, a component library, or a second icon pack.

## 2. What to use / what not to pull in

- Own CSS variables and modular sheets under `src/styles/` (`02-layout.css` through `08-power-tools.css`, plus `layout/`, `components/`, `modals/`).
- Terminal: `@xterm/xterm` and `@xterm/addon-fit` already in `package.json`. Keep the xterm palette aligned with `--mac-bg` / `--on-surface` (today the drawer hardcodes `#111214` in `terminalDrawerController.ts`).
- Motion recipes: https://transitions.dev ships HTML/CSS (custom license: free use, no redistribution; some items Pro/paid). Port the CSS into `src/styles/`; do not add the library or a React wrapper.
- Inspiration only (marketing/docs, not app chrome): https://styles.refero.design, https://cta.gallery, https://footer.design.
- **Not allowed without owner approval:** paid shader/3D packs (Scroll Shaders, Scrolltide, HorizonX). If approved, marketing only (`assets/showcase/`), with a static image fallback.
- **Never:** React kits (Aceternity, Astryx, ObsidianUI, affectUI, BoardUI, shadcn). They do not apply here.

## 3. Anti-slop rules (hard)

- Native feel first: dense, keyboard-driven, no landing-page heroes, glow blobs, or marketing layout inside the Tauri window.
- Do not add purple/blue gradient heroes, gradient text, or extra glow. The sidebar wordmark in `src/styles/layout/top.css` already uses gradient text, `backdrop-filter`, and accent bloom — do not copy that onto lists, dialogs, or panes.
- Do not add new `backdrop-filter` / glass except where overlays already use it (`src/styles/modals/base.css`, `src/styles/components/overlays.css`). No random glass on rows or the sidebar.
- One accent at a time (`--primary`). Do not paint chrome in `--mac-purple` unless the user chose the `purple` tray.
- No filler copy in dialogs, empty states, or the status bar. Say the action in plain words. No lorem, placeholders, or stock illustrations. Real screenshots belong in `assets/showcase/`.
- Body contrast `--on-surface` on `--mac-bg` must stay ≥ 4.5:1 (WCAG 2.2 AA). Do not use `--on-surface-variant` (`#8e8e93`) for primary labels.
- New motion: transform/opacity only, 150–300 ms, with `prefers-reduced-motion: reduce`.
- Hidden scrollbars (`src/styles/07-scrollbars.css`) are intentional; do not add overlay scrollbar skins.
- `assets/showcase/` is marketing, not the app. Do not import Plus Jakarta Sans, glow orbs, or a second palette into `src/`.

## 4. Review checklist (before saying "done")

- [ ] Screenshot the window: default Graphite Blue tray, plus one other tray (`ocean` or `mono`). Wide dual-pane and a ~900 px window. The app has no light theme and is not a 375 px phone UI.
- [ ] No new §3 violations (`linear-gradient` on text/heroes, extra `backdrop-filter`, emoji icons, web fonts in `src/`).
- [ ] Colours from `01-vars.css`; new icons only in `iconDefinitions.ts` / `menuIcons.ts`.
- [ ] Keyboard: focus visible, Tab order sane, shortcut still matches `docs/SHORTCUTS.md` (or the hotkey registry).
- [ ] Reduced motion: non-essential animation off.
- [ ] Empty, error, and no-selection states use real copy.
- [ ] If `assets/showcase/` changed: Lighthouse accessibility ≥ 95.
- [ ] Copied snippet: source URL + license in a comment.
