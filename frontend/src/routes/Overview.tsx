import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Bot, Shield, ScrollText, Play, Square, RotateCw } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useBotStore } from '@/stores/botStore'
import { useCaptureStore } from '@/stores/captureStore'
import { useConfigStore } from '@/stores/configStore'
import { invoke } from '@/lib/tauri'
import { AkagiWordmark } from '@/components/BrandLogo'
import { FullAutoDialog } from '@/components/FullAutoDialog'
import { OverlayToggle } from '@/components/OverlayToggle'

const DOT: Record<string, string> = {
  ready:    'bg-emerald-500',
  running:  'bg-emerald-500',
  loading:  'bg-amber-500',
  starting: 'bg-amber-500',
  idle:     'bg-zinc-500',
  stopped:  'bg-zinc-500',
  error:    'bg-red-500',
}

export function Overview() {
  const { t } = useTranslation()
  const bot = useBotStore((s) => s.status)
  const capture = useCaptureStore((s) => s.status)
  const logDir = useConfigStore((s) => s.logDir)
  const platformKind = useConfigStore((s) => s.config?.platform.kind)

  const captureTitle = 'kind' in capture && capture.kind === 'chromium' ? t('overview.capture_chromium') : t('overview.capture_mitm')
  const captureDetail = 'descriptor' in capture && capture.descriptor ? capture.descriptor : '—'

  return (
    <div className="p-6 flex flex-col gap-6 w-full">
      <div className="flex justify-center pt-2 pb-1">
        <AkagiWordmark className="h-[4.48rem]" />
      </div>

      {/* Title + blurb share a row with the two primary actions, which sit
          right-aligned and bottom-aligned with the live-status line. */}
      <div className="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <header>
          <h1 className="text-2xl font-semibold">{t('overview.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('overview.description')}</p>
        </header>
        <div className="flex flex-wrap items-center gap-2">
          <OverlayToggle />
          {platformKind === 'RiichiCity' && <FullAutoDialog />}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <StatusCard
          icon={Bot}
          title={t('overview.bot_card_title')}
          state={bot.state}
          detail={'bot' in bot && bot.bot ? bot.bot : '—'}
          extra={'actor_id' in bot ? `actor_id ${bot.actor_id}` : 'error' in bot ? bot.error : undefined}
        />
        <Card>
          <CardHeader className="flex flex-row items-center gap-2">
            <ScrollText className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm uppercase tracking-wider">{t('overview.log_session')}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="font-mono text-xs break-all">{logDir || '—'}</div>
          </CardContent>
        </Card>
      </div>

      {/* Capture drives the backend and needs its three controls side by
          side, so it takes the full row instead of sharing one. */}
      <CaptureCard
        title={captureTitle}
        state={capture.state}
        detail={captureDetail}
        error={'error' in capture ? capture.error : undefined}
      />
    </div>
  )
}

function StatusCard({
  icon: Icon, title, state, detail, extra,
}: {
  icon: typeof Bot
  title: string
  state: string
  detail: string
  extra?: string
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <CardTitle className="text-sm uppercase tracking-wider">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-2">
          <span className={`h-2 w-2 rounded-full ${DOT[state] ?? 'bg-zinc-500'}`} />
          <span className="capitalize text-sm font-medium">{state}</span>
        </div>
        <div className="text-xs font-mono text-muted-foreground mt-1">{detail}</div>
        {extra && <div className="text-xs text-muted-foreground mt-1">{extra}</div>}
      </CardContent>
    </Card>
  )
}

// Capture status + controls (start / restart / stop). The tile this replaced
// lived on the removed Game dashboard; Overview is the only screen left that
// needs to drive the capture backend.
function CaptureCard({
  title, state, detail, error,
}: {
  title: string
  state: string
  detail: string
  error?: string
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)

  const call = async (cmd: 'start_capture' | 'stop_capture' | 'restart_capture') => {
    setBusy(true)
    try {
      await invoke(cmd)
    } catch {
      /* surfaced via notify */
    } finally {
      setBusy(false)
    }
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center gap-2">
        <Shield className="h-4 w-4 text-muted-foreground" />
        <CardTitle className="text-sm uppercase tracking-wider">{title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <span className={`h-2 w-2 rounded-full ${DOT[state] ?? 'bg-zinc-500'}`} />
          <span className="capitalize text-sm font-medium">{state}</span>
          <span
            className="text-xs font-mono text-muted-foreground ml-auto truncate max-w-[60%]"
            title={detail}
          >
            {detail}
          </span>
        </div>
        {error && <div className="text-xs text-red-400 font-mono">{error}</div>}
        <div className="flex gap-1.5">
          <Button
            variant="default"
            size="sm"
            className="flex-1 gap-1.5"
            onClick={() => call('start_capture')}
            disabled={busy || state === 'running' || state === 'starting'}
          >
            <Play className="h-3.5 w-3.5" />
            {t('common.start')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="flex-1 gap-1.5"
            onClick={() => call('restart_capture')}
            disabled={busy}
          >
            <RotateCw className="h-3.5 w-3.5" />
            {t('common.restart')}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            className="flex-1 gap-1.5"
            onClick={() => call('stop_capture')}
            disabled={busy || state === 'stopped'}
          >
            <Square className="h-3.5 w-3.5" />
            {t('common.stop')}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
