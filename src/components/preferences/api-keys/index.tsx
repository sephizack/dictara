import { error as logError } from '@tauri-apps/plugin-log'
import { useAppConfig } from '@/hooks/useAppConfig'
import { useSaveAppConfig } from '@/hooks/useSaveAppConfig'
import { useEffect, useRef, useState } from 'react'
import { ProviderList } from './ProviderList'
import type { Provider } from './types'
import { Cpu } from 'lucide-react'
import { useAvailableModels } from '@/hooks/useLocalModels'

export function ApiKeys() {
  // Load app config using TanStack Query (type-safe via tauri-specta)
  const { data: config, isLoading } = useAppConfig()
  const saveConfig = useSaveAppConfig()
  const { data: models } = useAvailableModels()
  const residentModels = models?.filter((model) => model.isLoaded || model.isLoading) ?? []
  const estimatedRamMb = residentModels.reduce((total, model) => total + model.estimatedRamMb, 0)

  // Local state - initialized from config on first load
  const [activeProvider, setActiveProvider] = useState<Provider>(null)
  const [expandedSection, setExpandedSection] = useState<Provider>(null)
  const isInitialized = useRef(false)

  // Sync state from config on initial load only
  useEffect(() => {
    if (config && !isInitialized.current) {
      setActiveProvider(config.activeProvider)
      setExpandedSection(config.activeProvider)
      isInitialized.current = true
    }
  }, [config])

  // Toggle section expand/collapse (visual only)
  const handleToggleExpand = (provider: Provider) => {
    setExpandedSection(expandedSection === provider ? null : provider)
  }

  // Toggle provider activation (functional)
  const handleToggleProvider = (provider: Provider) => {
    // If clicking the already-active provider, disable it
    const newProvider = activeProvider === provider ? null : provider
    const previousProvider = activeProvider

    // Update local state immediately (optimistic update)
    setActiveProvider(newProvider)

    saveConfig.mutate(
      { activeProvider: newProvider },
      {
        onError: (e) => {
          // Revert on error
          setActiveProvider(previousProvider)
          logError(`[ApiKeys] Failed to update active provider: ${e}`)
        },
      }
    )
  }

  if (isLoading) {
    return <div className="text-muted-foreground">Loading...</div>
  }

  return (
    <div className="space-y-4">
      <div className="border-b pb-3 space-y-1">
        <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
          <span
            className="flex items-center gap-2"
            title="Estimated model memory only; excludes application overhead and varies with recording length."
          >
            <Cpu className="h-4 w-4" />
            Total model RAM (estimated)
          </span>
          <span className="font-medium tabular-nums">~{estimatedRamMb.toLocaleString()} MB</span>
        </div>
        {residentModels.map((model) => (
          <div
            key={model.name}
            className="flex flex-wrap justify-between gap-x-3 text-xs text-muted-foreground"
          >
            <span>
              {model.displayName}
              {model.isLoading ? ' (loading)' : ''}
            </span>
            <span className="tabular-nums">~{model.estimatedRamMb.toLocaleString()} MB</span>
          </div>
        ))}
      </div>
      <p className="text-sm text-muted-foreground">
        Select a transcription provider. Only one provider can be enabled at a time.
      </p>

      <ProviderList
        activeProvider={activeProvider}
        expandedSection={expandedSection}
        onToggleExpand={handleToggleExpand}
        onToggleActive={handleToggleProvider}
      />

      <p className="text-xs text-muted-foreground pt-4 border-t">
        API keys are stored securely in the macOS Keychain. Local models run entirely on your
        device.
      </p>
    </div>
  )
}
