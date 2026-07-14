import { createFileRoute } from '@tanstack/react-router'
import { useState, useEffect } from "react"
import { SessionRunner } from "@/components/session-runner"
import { SessionContextHeader, type MatchedSoulInfo } from "@/components/session-context-header"
import { fetchSessionDetail, fetchSoul } from "@/lib/api"

export const Route = createFileRoute('/possess/$sessionId')({
  component: SessionPage,
})

function SessionPage() {
  const { sessionId } = Route.useParams()
  const search = Route.useSearch()
  const mode = (search as { mode?: string }).mode || "single"
  const [sessionDone, setSessionDone] = useState(false)
  const [taskTitle, setTaskTitle] = useState("")
  const [matchedSouls, setMatchedSouls] = useState<MatchedSoulInfo[]>([])

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const detail = await fetchSessionDetail(sessionId, true)
        if (cancelled) return
        setTaskTitle(detail.session.title)

        const soulNames = Array.from(
          new Set(detail.messages.filter((m) => m.soul_name && m.role !== "system").map((m) => m.soul_name!))
        )
        const souls: MatchedSoulInfo[] = []
        for (const name of soulNames) {
          try {
            const profile = await fetchSoul(name)
            souls.push({
              name,
              field: profile.field || "",
              ismism_code: profile.ismism_code || "",
              rationale: profile.self_declare || "",
            })
          } catch {
            souls.push({ name, field: "", ismism_code: "", rationale: "" })
          }
        }
        if (!cancelled) setMatchedSouls(souls)
      } catch {}
    }
    load()
    return () => { cancelled = true }
  }, [sessionId])

  return (
    <div className="max-w-5xl mx-auto space-y-4" data-testid="session-page">
      {taskTitle && (
        <SessionContextHeader
          task={taskTitle}
          mode={mode}
          matchedSouls={matchedSouls}
        />
      )}
      <SessionRunner
        sessionId={sessionId}
        mode={mode}
        matchedSouls={matchedSouls}
        taskTitle={taskTitle}
        onDone={() => setSessionDone(true)}
        sessionDone={sessionDone}
      />
    </div>
  );
}
