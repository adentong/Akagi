import { useEffect } from 'react'
import { HAS_TAURI, invoke, listen } from '@/lib/tauri'
import type {
  BotStatus,
  CaptureStatus,
  MjaiEvent,
  Notification,
  OverlayConfig,
  Snapshot,
} from '@/types'
import { useBotStore } from '@/stores/botStore'
import { useCaptureStore } from '@/stores/captureStore'
import { useApiStatusStore } from '@/stores/apiStatusStore'
import { useInstallStore } from '@/stores/installStore'
import { useConfigStore } from '@/stores/configStore'
import { toast, type ToastSeverity } from '@/components/ui/sonner'

// Backend `Notification.level` ∈ {info,success,warn,error}; toast helper
// uses `warning`. Map across.
const TOAST_SEVERITY: Record<Notification['level'], ToastSeverity> = {
  info: 'info',
  success: 'success',
  warn: 'warning',
  error: 'error',
}

// One-shot bridge mounted from <App>. Subscribes to all Tauri events,
// hydrates initial state, and unsubscribes on unmount.
export function useTauriBridge() {
  useEffect(() => {
    if (!HAS_TAURI) return

    const unlistens: Array<() => void> = []
    let cancelled = false

    ;(async () => {
      try {
        const status = await invoke<Snapshot>('get_status')
        if (cancelled) return
        useConfigStore.getState().setConfig(status.config)
        useConfigStore.getState().setLogDir(status.log_dir)
        useBotStore.getState().setStatus(status.bot_status)
        useCaptureStore.getState().set(status.capture_status)
      } catch {
        /* ignore */
      }
    })()

    listen<MjaiEvent>('mjai-event', (e) => {
      // Fresh game: clear any lingering online-API outage from the last one.
      if (e.type === 'start_game') useApiStatusStore.getState().reset()
    }).then((u) => unlistens.push(u))

    listen<BotStatus>('bot-status', (s) => {
      useBotStore.getState().setStatus(s)
    }).then((u) => unlistens.push(u))

    listen<CaptureStatus>('capture-status', (s) => {
      useCaptureStore.getState().set(s)
    }).then((u) => unlistens.push(u))

    // The overlay window can turn itself off (its × button). Mirror that back
    // into the config store so the Overview toggle doesn't keep claiming the
    // overlay is open.
    listen<OverlayConfig>('overlay-config', (o) => {
      useConfigStore.getState().setOverlay(o)
    }).then((u) => unlistens.push(u))

    listen<Notification>('notify', (n) => {
      // Drive the persistent "Online API" health indicator (Statusbar) off the
      // same channel the backend uses for degrade/recover toasts.
      if (n.id === 'native-api-health') {
        useApiStatusStore
          .getState()
          .setDegraded(n.level === 'warn' || n.level === 'error', n.body)
      }
      // Feed env install/sync progress to the blocking overlay (which is
      // shown for the duration of `withInstallBlock`). Ids: `bot-install-*`
      // (GitHub install/reinstall) and `bot-sync-*` ("Reinstall environment").
      if (n.id && (n.id.startsWith('bot-install-') || n.id.startsWith('bot-sync-'))) {
        useInstallStore.getState().setProgress(n)
      }
      toast[TOAST_SEVERITY[n.level]](n.title, {
        description: n.body,
        id: n.id,
        ...(n.sticky ? { duration: Infinity } : {}),
      })
    }).then((u) => unlistens.push(u))

    return () => {
      cancelled = true
      unlistens.forEach((u) => u())
    }
  }, [])
}
