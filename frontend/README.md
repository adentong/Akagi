# frontend (React + Vite + shadcn/ui)

The Akagi UI: a Tauri webview that renders the Overview / Bots / Settings
pages and the always-on-top suggestion overlay.

## Stack

- Vite + React 19 + TypeScript
- Tailwind CSS v4 + shadcn/ui (Radix Nova preset)
- react-router-dom (HashRouter) — routes for Overview / Bots / Settings / Setup
- zustand — app state, sliced by domain (bot, capture, config, notify, theme, …)
- react-i18next — `en` / `ja` / `zh-TW` / `zh-CN`
- @tauri-apps/api — invoke + listen wrappers (`src/lib/tauri.ts`)
- mahgen — `<mah-gen>` web component, wrapped by `src/components/Mahgen.tsx`

## Layout

```
src/
  App.tsx                    Sidebar + routes + statusbar
  main.tsx                   HashRouter / overlay root + i18n init
  index.css                  Tailwind v4 + theme tokens
  components/
    Sidebar/                 Left nav, brand, language selector (components/sidebar/)
    Statusbar.tsx            Bottom strip
    OverlayToggle.tsx        Overview action-row overlay switch
    FullAutoDialog.tsx       Riichi City full-auto session dialog
    Mahgen.tsx               <mah-gen> React wrapper
    BotShowList.tsx          Ranked suggestion rows (overlay + dialog)
    ui/                      shadcn primitives (copy-pasted; edit freely)
  routes/
    Overview.tsx             Status cards + capture controls + action row
    Bots.tsx                 Table + install + per-bot settings drawer
    Settings.tsx             AppConfig form
    Setup.tsx                First-run wizard
    Overlay.tsx              The overlay window's entire UI
  stores/                    zustand stores (bot, capture, config, theme, …)
  hooks/
    useTauriBridge.ts        One-shot Tauri event subscription, mounted from <App>
  i18n/
    index.ts                 i18next bootstrap
    resources/{en,ja,zh-TW,zh-CN}.json
  lib/
    tauri.ts                 invoke + listen wrappers
    mahgenRegistry.ts        <mah-gen> sizing registry
    tileIdx.ts               mjai tiles → mahgen DSL
    botShow.ts               Pure helpers for the bot's `meta.show` payload
    utils.ts                 shadcn `cn`
  types.ts                   Schema mirror (MjaiEvent, AppConfig, …)
```

## The overlay window

Both windows load the same `index.html`; `main.tsx` branches on
`getCurrentWindow().label` (`isOverlayWindow()`) to mount either the router
or the bare `Overlay` root. Keep that check and `overlay::LABEL` in step —
the capability file is scoped to the label.

## Scripts

```
npm ci                  # install from the lockfile
npm run dev             # Vite dev server on :1420 (intended for `tauri dev`)
npm run build           # tsc -b && vite build → dist/
npm test                # vitest (jsdom)
npm run lint            # eslint
npm run preview         # serve dist/ for a sanity check
```

## i18n

Strings live in `src/i18n/resources/*.json`; keep the four locales in sync
(the same keys in every file). Dynamic keys built with template literals
(`status.stage_${…}`, `settings.mode_${…}`) must exist for every value the
code can produce.
