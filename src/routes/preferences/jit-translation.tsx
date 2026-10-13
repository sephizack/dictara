import { createFileRoute } from '@tanstack/react-router'
import { JitTranslation } from '@/components/preferences/JitTranslation'

export const Route = createFileRoute('/preferences/jit-translation')({
  component: JitTranslation,
})
