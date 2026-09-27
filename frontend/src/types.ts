// Mirrors backend schema. Source: frontend/README.md.

export type MjaiEvent =
  | { type: 'start_game'; names: string[]; kyoku_first?: number; aka_flag?: boolean; id?: number; num_players?: number }
  | { type: 'start_kyoku'; bakaze: string; dora_marker: string; kyoku: number; honba: number; kyotaku: number; oya: number; scores: number[]; tehais: string[][]; num_players?: number }
  | { type: 'tsumo'; actor: number; pai: string }
  | { type: 'dahai'; actor: number; pai: string; tsumogiri: boolean }
  | { type: 'chi'; actor: number; target: number; pai: string; consumed: [string, string] }
  | { type: 'pon'; actor: number; target: number; pai: string; consumed: [string, string] }
  | { type: 'daiminkan'; actor: number; target: number; pai: string; consumed: [string, string, string] }
  | { type: 'kakan'; actor: number; pai: string; consumed: [string, string, string] }
  | { type: 'ankan'; actor: number; consumed: [string, string, string, string] }
  | { type: 'dora'; dora_marker: string }
  | { type: 'reach'; actor: number; pai?: string }
  | { type: 'reach_accepted'; actor: number }
  | { type: 'hora'; actor: number; target: number; deltas?: number[]; ura_markers?: string[] }
  | { type: 'ryukyoku'; deltas?: number[] }
  | { type: 'kita'; actor: number; pai?: string }
  | { type: 'end_kyoku' }
  | { type: 'end_game' }
  | { type: 'none' }

export type BotResponse = MjaiEvent & { meta?: Record<string, unknown> }

/** Bot-driven custom display payload, attached on `meta.show`.
 *  Schema is intentionally generic so a single tile can render top-N
 *  actions, opponent reads, yaku breakdowns, etc. — bots decide the
 *  semantics and formatting. */
export type ShowItem = {
  /** Primary text on the row. */
  label?: string
  /** mjai tile strings; converted to mahgen via `mjaiToMahgen`. */
  pais?: string[]
  /** Raw mahgen DSL string. Wins over `pais` if both are set. */
  tiles?: string
  /** Right-side text (any format — e.g. "85.42%", "+12000"). */
  value?: string
  /** Hex accent color (e.g. "#00ff80") — applied as left bar + faint row tint. */
  color?: string
  /** Small subtitle under `label`. */
  note?: string
}

export type ShowMeta = {
  /** Optional title; falls back to the tile's default title. */
  title?: string
  items: ShowItem[]
}

export type BotStatus =
  | { state: 'idle' }
  | { state: 'loading'; bot: string; stage: 'syncing_deps' | 'spawning' }
  | { state: 'ready'; bot: string; actor_id: number }
  | { state: 'error'; bot: string; error: string }
  | { state: 'stopped'; bot: string }

export type CaptureKind = 'mitm' | 'chromium'

export type CaptureStatus =
  | { state: 'stopped' }
  | { state: 'starting'; kind: CaptureKind; descriptor: string }
  | { state: 'running'; kind: CaptureKind; descriptor: string }
  | { state: 'error'; kind: CaptureKind; descriptor?: string; error: string }

export type Notification = {
  level: 'info' | 'success' | 'warn' | 'error'
  title: string
  body?: string
  sticky: boolean
  id?: string
}

export type CaptureMode = 'mitm' | 'chromium'

export type ChromiumConfig = {
  executable: string
  user_data_dir: string
  start_url: string
  cft_channel: string
  force_cft: boolean
  extra_args: string[]
}

export type CaptureConfig = {
  mode: CaptureMode
  chromium: ChromiumConfig
}

export type DetectedBrowser = {
  kind: 'chrome' | 'edge' | 'brave' | 'chromium' | 'chrome_for_testing'
  path: string
}

/// Bridge selector (the runtime kind). Mirrors
/// `src/config/platform.rs::Platform` (`#[derive(Serialize)]` →
/// PascalCase JSON: `"Majsoul"`, `"Tenhou"`).
export type PlatformKind = 'Majsoul' | 'Tenhou' | 'RiichiCity'

export type MajsoulAutoplayConfig = {
  pre_click_delay_min_ms: number
  pre_click_delay_max_ms: number
  inter_click_delay_ms: number
  hover_delay_ms: number
  click_hold_ms: number
  /** Wait this long for the client's own input command after a click
   *  before pressing again; 0 disables verification. */
  verify_input_ms: number
  /** Retries when no input command follows a click sequence; 0 = log only. */
  click_retries: number
  dealer_first_discard_extra_delay_ms: number
}

/** Pre-click delay model parameters. Mirrors
 *  `src/config/autoplay.rs::DelayModelConfig`; the fine-grained knobs are
 *  config-file-only — the Settings UI exposes the Lua script fields. */
export type DelayMode = 'legacy' | 'lua'

export type DelayModelConfig = {
  /** Which policy is active; exactly one. `legacy` = the old fixed
   *  uniform model, `lua` = the scriptable human-like model backed by
   *  `delay.lua` next to the config file (auto-generated). */
  mode: DelayMode
  /** UI-readiness floor: minimum total thinking time per decision, ms.
   *  Clicks issued before Mahjong Soul renders the UI are lost. */
  min_delay_ms: number
  /** Higher floor for decisions that click an action button (chi/pon/
   *  kan/ron/skip/riichi) — buttons render after the discard animation
   *  plus their own pop-in, later than hand tiles. */
  min_button_delay_ms: number
  distribution: 'uniform' | 'log_normal'
  /** Per-decision-kind log-normal `[mu, sigma]` in ln(seconds); keys like
   *  `dahai_tedashi`, `claim`. Calibrated from ranked-game records. */
  lognormal: Record<string, [number, number]>
  bank_on_long_thought: boolean
  riichi_extra_ms: number
  kan_extra_ms: number
  safety_margin_ms: number
  bank_use_fraction: number
  bank_max_single_ms: number
  no_budget_cap_ms: number
}

/** Riichi City ranked rooms, lowest to highest. Mirrors
 *  `src/config/autoplay.rs::RiichiRoom`. */
export type RiichiRoom = 'star' | 'moon' | 'sun' | 'galaxy'

/** Riichi City ranked game lengths. Mirrors
 *  `src/config/autoplay.rs::RiichiGameType`. */
export type RiichiGameType = 'east_only' | 'hanchan'

/** Riichi City autoplay session knobs. Mirrors
 *  `src/config/autoplay.rs::RiichiCityAutoplayConfig`. */
export type RiichiCityAutoplayConfig = {
  /** Room to queue ranked matches in. */
  room: RiichiRoom
  /** Game length to queue. */
  game_type: RiichiGameType
  /** Galaxy only: accept a Sun table if none is found within 2 minutes. */
  galaxy_fallback_sun: boolean
  /** Wait after a game ends before queueing the next (actual wait is
   *  uniform in [x, 1.5x]), ms. */
  inter_game_delay_ms: number
}

export type AutoplayConfig = {
  enabled: boolean
  majsoul: MajsoulAutoplayConfig
  delay: DelayModelConfig
  riichi_city: RiichiCityAutoplayConfig
}

/** Optional cloud-inference settings for the built-in native bot.
 *  Mirrors `crate::config::NativeApiConfig`. */
export type NativeApiConfig = {
  enabled: boolean
  base_url: string
  key: string
  model_4p: string
  model_3p: string
  /** Whether `proxy` is applied. Off ⇒ direct even if `proxy` holds a value. */
  proxy_enabled: boolean
  /** Proxy for all inference-server traffic: http://, https://, socks5:// or
   *  socks5h:// URL. Applied only when `proxy_enabled`; empty = direct. */
  proxy: string
  /** Per-decision timeout for POST /v3/react, in milliseconds. Clamped to
   *  500–10000ms on the backend before use. Default 3000. */
  react_timeout_ms: number
}

/** The always-on-top suggestion overlay. Mirrors `crate::config::OverlayConfig`. */
export type OverlayConfig = {
  enabled: boolean
  top_n: number
  opacity: number
  always_on_top: boolean
}

/** How GitHub-hosted downloads are routed. Mirrors
 *  `crate::config::GithubMirrorMode` (serde snake_case). */
export type GithubMirrorMode = 'auto' | 'direct' | 'mirror'

/** `[network]` section. Mirrors `crate::config::NetworkConfig`. */
export type NetworkConfig = {
  github_mirror_mode: GithubMirrorMode
  /** gh-proxy-style accelerator prefix (e.g. `https://gh-proxy.com`);
   *  tried before the built-in mirror list. Empty = unset. */
  github_custom_mirror: string
}

/** Bounds enforced by `crate::config::overlay` — mirrored so the UI can't
 *  offer a value the backend would silently clamp. */
export const OVERLAY_TOP_N_MIN = 1
export const OVERLAY_TOP_N_MAX = 5
export const OVERLAY_OPACITY_MIN = 0.3
export const OVERLAY_OPACITY_MAX = 1.0

export type AppConfig = {
  general: { first_run_completed: boolean; developer_mode: boolean }
  logging: { dir: string; level: string; all_level: string }
  platform: { kind: PlatformKind }
  proxy: { enabled: boolean; addr: string; ca_dir: string; block_telemetry: boolean }
  bot: {
    enabled: boolean
    active_4p: string
    active_3p: string
    auto_sync: boolean
    dir: string
    api: NativeApiConfig
  }
  capture: CaptureConfig
  autoplay: AutoplayConfig
  overlay: OverlayConfig
  network: NetworkConfig
}

// ---------- Built-in bot cloud inference (native API) ----------
// Mirror the response shapes from `crate::bot::api`.

/** `GET /v3/key` — a key's plan, expiry and live limits. */
export type KeyStatus = {
  plan: string
  expires_at: string
  usage_today: number
  rpd: number
  rpm: number
  topk: number
}

/** One model a key's plan may use (`GET /v3/models`). */
export type ModelInfo = { id: string; game: string; desc: string }

/** `POST /v3/redeem` result. `key` is present only when a new key is minted. */
export type RedeemResponse = {
  key?: string | null
  key_last4: string
  plan: string
  expires_at: string
  extended: boolean
}

/**
 * `GET /healthz` — liveness + aggregate load. Nothing about the model
 * registry is exposed here (models come from the authenticated `/v3/models`);
 * `status` is `"degraded"` when any model worker is down.
 */
export type ApiHealth = {
  status: string
  /** Total pending + in-flight inference rows. */
  queue_depth: number
  workers_alive: boolean
}

// ---------- Self-serve key purchase (PayPal) ----------
// Mirror the response shapes from `crate::bot::purchase`.

/** `POST /paypal/create-order` — a pending one-time purchase. */
export type CreatedOrder = {
  order_id: string
  approve_url: string
  claim_secret: string
}

/** `POST /paypal/create-subscription` — a pending subscription. */
export type CreatedSubscription = {
  subscription_id: string
  approve_url: string
  claim_secret: string
}

/**
 * `POST /creem/create-checkout` — a pending Creem checkout. One create
 * endpoint serves both one-time and subscription products; the poll
 * (`POST /creem/result`) reuses the `OrderResult` shape for both kinds
 * (a subscription resolves to `key` with `days: 0`).
 */
export type CreatedCheckout = {
  checkout_id: string
  checkout_url: string
  claim_secret: string
}

/**
 * One poll of `POST /paypal/order-result`. On `status: ready` exactly one of
 * `key` / `code` is set: `key` when the order was created with `redeem: true`
 * (the server already spent the code), `code` otherwise. Branch on whichever
 * is present — never re-redeem a code that came back alongside a key.
 */
export type OrderResult = {
  status: string
  code?: string | null
  key?: string | null
  plan?: string | null
  days?: number | null
}

/** One poll of `POST /paypal/subscription-result`. `key` only on `ready`. */
export type SubscriptionResult = {
  status: string
  key?: string | null
  plan?: string | null
  next_billing?: string | null
}

export type FieldKind = 'string' | 'bool' | 'int' | 'float' | 'enum'

export type FieldSpec = {
  type: FieldKind
  label: string
  default: unknown
  help?: string
  secret?: boolean
  min?: number
  max?: number
  step?: number
  choices?: string[]
}

export type Manifest = {
  manifest_version: number
  bot: {
    name: string
    display?: string
    description?: string
    version?: string
    /** Game modes this bot can play. Backend defaults to `["4p"]` when absent. */
    supported_modes: string[]
  }
  source?: { type: 'github_release'; repo: string; asset_glob?: string }
  settings: Record<string, FieldSpec>
}

export type BotInfo = {
  name: string
  dir: string
  has_pyproject: boolean
  /** Bot's Python environment is installed and ready (no slow first-spawn sync). */
  env_ready: boolean
  manifest?: Manifest
}

export type BotSettings = {
  manifest: Manifest
  values: Record<string, unknown>
}

export type Snapshot = {
  config: AppConfig
  bot_status: BotStatus
  capture_status: CaptureStatus
  log_dir: string
}

