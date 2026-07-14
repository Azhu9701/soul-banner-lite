import { createFileRoute } from '@tanstack/react-router'
import { KnowledgeBrowser } from "@/components/knowledge-browser"

export const Route = createFileRoute('/knowledge')({
  component: KnowledgePage,
})

function KnowledgePage() {
  return (
    <div className="flex flex-col gap-6 p-6 max-w-4xl mx-auto">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">知识库</h1>
        <p className="text-sm text-muted-foreground mt-1">
          以问题为单位的分析报告、知识卡片和全文检索
        </p>
      </div>
      <KnowledgeBrowser />
    </div>
  );
}
