import { createFileRoute, notFound } from '@tanstack/react-router'
import { useEffect, useState } from "react"
import { fetchSoul, type SoulProfile } from "@/lib/api"
import { SoulPrompt } from "@/components/soul-prompt"
import { PracticeObservations } from "@/components/practice-observations"
import { SummonButton } from "@/components/summon-button"
import { SoulModelConfig } from "@/components/soul-model-config"
import { DeleteSoulButton } from "@/components/delete-soul-button"
import { Calendar } from "lucide-react"
import { Skeleton } from "@/components/ui/skeleton"

export const Route = createFileRoute('/souls/$name')({
  component: SoulDetailPage,
})

function SoulDetailPage() {
  const { name } = Route.useParams()
  const decodedName = decodeURIComponent(name)
  const [profile, setProfile] = useState<SoulProfile | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    fetchSoul(decodedName).then(setProfile).catch(() => setError(true))
  }, [decodedName])

  if (error) throw notFound()
  if (!profile) return <Skeleton className="h-96" />

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h1 className="text-2xl font-bold truncate">{profile.name}</h1>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <SummonButton soulName={profile.name} />
          <DeleteSoulButton soulName={profile.name} variant="text" />
        </div>
      </div>

      <div className="flex items-center gap-1 text-xs text-muted-foreground">
        <Calendar className="h-3 w-3 shrink-0" />
        创建于 {new Date(profile.created_at).toLocaleDateString("zh-CN")}
      </div>

      {profile.domains.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {profile.domains.map((d) => (
            <span key={d} className="rounded-md bg-muted px-2.5 py-1 text-xs">{d}</span>
          ))}
        </div>
      )}

      {profile.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {profile.tags.map((t) => (
            <span key={t} className="rounded-full border px-2.5 py-1 text-xs text-muted-foreground">{t}</span>
          ))}
        </div>
      )}

      <SoulPrompt prompt={profile.summon_prompt} soulName={profile.name} />

      <SoulModelConfig soulName={profile.name} currentModel={profile.model} />

      <PracticeObservations observations={profile.practice_observations || []} />
    </div>
  );
}
