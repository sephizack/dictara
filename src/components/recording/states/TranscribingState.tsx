import { Mirage } from 'ldrs/react'
import 'ldrs/react/Mirage.css'

export function TranscribingState({ label }: { label?: string }) {
  return (
    <div className="flex flex-col gap-2 w-full h-full justify-center items-center" role="status">
      <Mirage size="60" speed="2.5" color="#9ca3af" />
      {label && <span className="text-xs text-gray-300">{label}</span>}
    </div>
  )
}
