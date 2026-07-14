import { createFileRoute, Link } from '@tanstack/react-router'
import { useEffect, useState } from "react"
import { fetchSouls, type SoulListEntry } from "@/lib/api"
import { SoulCardGrid } from "@/components/soul-card-grid"
import { SoulFilterBar } from "@/components/soul-filter-bar"
import { Skeleton } from "@/components/ui/skeleton"
import { Button } from "@/components/ui/button"
import { Search, Wand2 } from "lucide-react"

export const Route = createFileRoute('/souls')({
  component: SoulListPage,
})

function SoulListPage() {
  const [souls, setSouls] = useState<SoulListEntry[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchSouls().then(setSouls).finally(() => setLoading(false))
  }, [])

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">角色档案</h1>
          <p className="text-sm text-muted-foreground mt-1">角色列表</p>
        </div>
        <div className="flex gap-2">
          <Link to="/souls/collect">
            <Button variant="outline" size="sm" data-testid="collect-soul-btn">
              <Search className="h-4 w-4 mr-1" />添加角色
            </Button>
          </Link>
          <Link to="/souls/refine">
            <Button variant="outline" size="sm" data-testid="refine-soul-btn">
              <Wand2 className="h-4 w-4 mr-1" />编辑角色
            </Button>
          </Link>
        </div>
      </div>
      {loading ? (
        <div className="space-y-4">
          <div className="flex gap-3">
            <Skeleton className="h-10 flex-1" />
            <Skeleton className="h-10 w-28" />
          </div>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {Array.from({ length: 8 }).map((_, i) => (
              <Skeleton key={i} className="h-36 rounded-lg" />
            ))}
          </div>
        </div>
      ) : (
        <>
          <SoulFilterBar totalCount={souls.length} />
          <SoulCardGrid souls={souls} />
        </>
      )}
    </div>
  );
}
