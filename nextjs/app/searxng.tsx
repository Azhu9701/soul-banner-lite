import { createFileRoute } from '@tanstack/react-router'
import { SearxngSearch } from "@/components/searxng-search"

export const Route = createFileRoute('/searxng')({
  component: SearxngPage,
})

function SearxngPage() {
  return (
    <div className="flex flex-col gap-6 p-6 max-w-3xl mx-auto">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">SearXNG 搜索</h1>
        <p className="text-sm text-muted-foreground mt-1">
          通过自托管 SearXNG 实例进行隐私安全的互联网搜索
        </p>
      </div>
      <SearxngSearch />
    </div>
  );
}
