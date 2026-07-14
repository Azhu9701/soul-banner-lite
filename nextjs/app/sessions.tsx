import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router'
import { useEffect, useState } from "react"
import { fetchSessions, type SessionSummary } from "@/lib/api"
import { SessionTimeline } from "@/components/session-timeline"
import { Skeleton } from "@/components/ui/skeleton"

export const Route = createFileRoute('/sessions')({
  component: SessionsLayout,
})

function SessionsLayout() {
  const location = useLocation()
  const isChildRoute = location.pathname !== '/sessions'

  if (isChildRoute) {
    return <Outlet />
  }

  return <SessionsListPage />
}

function SessionsListPage() {
  const [sessions, setSessions] = useState<SessionSummary[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchSessions(200).then(setSessions).finally(() => setLoading(false))
  }, [])

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">会话历史</h1>
        <p className="text-sm text-muted-foreground mt-1">浏览所有庭审会话记录</p>
      </div>
      {loading ? (
        <Skeleton className="h-96 rounded-xl" />
      ) : (
        <SessionTimeline sessions={sessions} />
      )}
    </div>
  );
}
