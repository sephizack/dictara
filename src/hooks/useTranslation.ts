import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { commands } from '@/bindings'
import { MODELS_QUERY_KEY } from './useLocalModels'

export const TRANSLATION_CONFIG_QUERY_KEY = ['translationConfig'] as const

export function useTranslationConfig() {
  return useQuery({
    queryKey: TRANSLATION_CONFIG_QUERY_KEY,
    queryFn: () => commands.loadTranslationConfig(),
    refetchInterval: 3000,
  })
}

export function useSaveTranslationConfig() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (enabled: boolean) => {
      const result = await commands.saveTranslationConfig(enabled)
      if (result.status === 'error') throw new Error(result.error)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: TRANSLATION_CONFIG_QUERY_KEY })
      queryClient.invalidateQueries({ queryKey: MODELS_QUERY_KEY })
    },
  })
}
