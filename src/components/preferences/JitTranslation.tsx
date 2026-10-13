import { useState } from 'react'
import { Download, Trash2, X, Loader2 } from 'lucide-react'
import type { ModelInfo } from '@/bindings'
import { useAppConfig } from '@/hooks/useAppConfig'
import {
  useAvailableModels,
  useDownloadModel,
  useCancelModelDownload,
  useDeleteModel,
} from '@/hooks/useLocalModels'
import { useTranslationConfig, useSaveTranslationConfig } from '@/hooks/useTranslation'
import { useShortcutsConfig } from '@/hooks/useShortcutsConfig'
import { useSaveShortcutsConfig } from '@/hooks/useSaveShortcutsConfig'
import { KeyCaptureInput } from '@/components/shortcuts/KeyCaptureInput'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { formatBytes } from '@/lib/utils'
import { ModelDownloadProgress, useDownloadProgress } from './api-keys/ModelDownloadProgress'

export function JitTranslation() {
  const { data: config, isLoading: configLoading } = useAppConfig()
  const { data: models, isLoading: modelsLoading } = useAvailableModels()

  if (configLoading || modelsLoading) {
    return <div className="text-muted-foreground text-sm">Loading...</div>
  }

  const translationModels = models?.filter((model) => model.isTranslation) ?? []

  return (
    <div className="space-y-6">
      {translationModels.length === 0 && (
        <p className="text-sm text-muted-foreground">No compatible translation model.</p>
      )}
      {translationModels.map((model) => (
        <TranslationSettings
          key={model.name}
          model={model}
          isActive={config?.activeProvider === 'local'}
        />
      ))}
    </div>
  )
}

function TranslationSettings({ model, isActive }: { model: ModelInfo; isActive: boolean }) {
  const { data: translation } = useTranslationConfig()
  const { data: shortcuts } = useShortcutsConfig()
  const saveTranslation = useSaveTranslationConfig()
  const saveShortcut = useSaveShortcutsConfig()
  const download = useDownloadModel()
  const cancel = useCancelModelDownload()
  const deleteModel = useDeleteModel()
  const progress = useDownloadProgress(model.name)
  const [error, setError] = useState<string | null>(null)
  const enabled = translation?.enabled ?? false
  const downloading = model.isDownloading || !!progress
  const pending = saveTranslation.isPending || model.isLoading || deleteModel.isPending

  const perform = async (action: () => Promise<unknown>) => {
    setError(null)
    try {
      await action()
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error))
    }
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <Label htmlFor="local-translation" className="text-base">
          Translate to English
        </Label>
        <Switch
          id="local-translation"
          checked={enabled}
          disabled={!translation || pending || (!enabled && (!model.isDownloaded || !isActive))}
          onCheckedChange={(checked) => perform(() => saveTranslation.mutateAsync(checked))}
        />
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
        <div className="min-w-0">
          <p className="font-medium">{model.displayName}</p>
          <p className="text-xs text-muted-foreground">
            {formatBytes(model.sizeBytes)} / ~{model.estimatedRamMb.toLocaleString()} MB RAM
          </p>
          <p className="text-xs text-muted-foreground" role="status">
            {pending ? 'Loading...' : model.isLoaded ? 'Loaded' : 'Not loaded'}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {pending && (
            <Loader2 className="h-4 w-4 animate-spin" aria-label="Updating translation" />
          )}
          {downloading ? (
            <Button
              size="sm"
              variant="outline"
              onClick={() => perform(() => cancel.mutateAsync(model.name))}
            >
              <X className="mr-1 h-4 w-4" />
              Cancel
            </Button>
          ) : model.isDownloaded ? (
            <Button
              size="icon"
              variant="ghost"
              title="Delete translation model"
              aria-label="Delete translation model"
              disabled={pending}
              onClick={() => perform(() => deleteModel.mutateAsync(model.name))}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              disabled={download.isPending}
              onClick={() => perform(() => download.mutateAsync(model.name))}
            >
              <Download className="mr-1 h-4 w-4" />
              Download
            </Button>
          )}
        </div>
      </div>
      {downloading && <ModelDownloadProgress progress={progress} />}
      {enabled && shortcuts && (
        <KeyCaptureInput
          label="English dictation shortcut"
          value={shortcuts.translate?.keys ?? []}
          onChange={(keys) =>
            perform(() => saveShortcut.mutateAsync({ ...shortcuts, translate: { keys } }))
          }
        />
      )}
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  )
}
