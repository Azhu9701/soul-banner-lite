import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useState, useEffect } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { ArrowLeft, Loader2, Sparkles, Check, Save, Wand2 } from "lucide-react"
import { API_BASE } from "@/lib/api"

export const Route = createFileRoute('/souls/refine')({
  component: RefinePage,
})

function RefinePage() {
  const navigate = useNavigate()
  const [material, setMaterial] = useState("")
  const [loading, setLoading] = useState(false)
  const [refined, setRefined] = useState<any>(null)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    const stored = sessionStorage.getItem("refine-material")
    if (stored) setMaterial(stored)
  }, [])

  const onRefine = async () => {
    if (!material.trim()) return
    setLoading(true)
    try {
      const r = await fetch(`${API_BASE}/souls/refine`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ raw_material: material }),
      })
      const data = await r.json()
      setRefined(data)
      sessionStorage.removeItem("refine-material")
    } finally {
      setLoading(false)
    }
  }

  if (!refined) {
    return (
      <div className="max-w-3xl mx-auto space-y-6">
        <div className="flex items-center gap-3">
          <Link to="/souls"><Button variant="ghost" size="icon"><ArrowLeft className="h-4 w-4" /></Button></Link>
          <div>
            <h1 className="text-2xl font-bold">炼化</h1>
            <p className="text-sm text-muted-foreground">Raw 素材 → 结构化 Soul Profile → 自动入幡</p>
          </div>
        </div>
        {!material && (
          <div className="rounded-lg bg-muted p-4 text-sm space-y-1">
            <p><strong>如何获取 Raw 素材？</strong></p>
            <p>1. 从 <Link to="/souls/collect" className="text-primary underline">收集角色</Link> 页面自动收集</p>
            <p>2. 手动粘贴以下格式的素材（生平/思想/方法论/代表作/影响）</p>
          </div>
        )}
        <div className="rounded-lg border p-4 space-y-2">
          <h3 className="text-sm font-semibold">Raw 素材</h3>
          <Textarea value={material} onChange={(e) => setMaterial(e.target.value)} rows={15} className="font-mono text-xs" data-testid="refine-material" placeholder="粘贴 raw 素材..." />
        </div>
        <Button onClick={onRefine} disabled={loading || !material.trim()} className="w-full" size="lg" data-testid="refine-btn">
          {loading ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Wand2 className="h-4 w-4 mr-1" />}
          {loading ? "炼化中..." : "开始炼化"}
        </Button>
      </div>
    )
  }

  const { profile, rationale } = refined

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="flex items-center gap-3">
        <Link to="/souls"><Button variant="ghost" size="icon"><ArrowLeft className="h-4 w-4" /></Button></Link>
        <div>
          <h1 className="text-2xl font-bold">炼化完成</h1>
          <p className="text-sm text-muted-foreground">{profile.name} — 已自动写入 registry</p>
        </div>
      </div>

      <div className="rounded-lg border p-6 space-y-4">
        <div>
          <h2 className="text-xl font-bold">{profile.name}</h2>
          <p className="text-sm font-mono text-muted-foreground">{profile.ismism_code} · {profile.field}</p>
        </div>

        <div className="grid grid-cols-2 gap-2 text-sm">
          <p><span className="text-muted-foreground">本体论：</span>{profile.ontology}</p>
          <p><span className="text-muted-foreground">认识论：</span>{profile.epistemology}</p>
          <p><span className="text-muted-foreground">目的论：</span>{profile.teleology}</p>

        </div>

        {profile.domains?.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {profile.domains.map((d: string) => (
              <span key={d} className="rounded-md bg-muted px-2 py-0.5 text-xs">{d}</span>
            ))}
          </div>
        )}

        <div>
          <h3 className="text-sm font-semibold mb-1">召唤词</h3>
          <div className="prose prose-sm max-w-none dark:prose-invert bg-muted rounded-md p-3 max-h-80 overflow-y-auto
            [&_h1]:text-base [&_h1]:font-bold [&_h1]:mt-3 [&_h1]:mb-2
            [&_h2]:text-sm [&_h2]:font-semibold [&_h2]:mt-3 [&_h2]:mb-1.5
            [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:mt-3 [&_h3]:mb-1.5
            [&_p]:my-1.5 [&_p]:leading-relaxed [&_p]:text-sm
            [&_ul]:my-1 [&_ol]:my-1
            [&_li]:my-1 [&_li]:text-sm [&_li]:leading-relaxed
            [&_strong]:font-semibold [&_strong]:text-foreground/90
            [&_code]:bg-muted-foreground/15 [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-xs
            [&_pre]:my-2 [&_pre]:p-3 [&_pre]:bg-muted-foreground/10 [&_pre]:rounded-lg [&_pre]:text-xs [&_pre]:overflow-x-auto
          ">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>
              {profile.summon_prompt}
            </ReactMarkdown>
          </div>
        </div>

        <div>
          <h3 className="text-sm font-semibold mb-1">ismism 理据</h3>
          <p className="text-sm text-muted-foreground">{rationale}</p>
        </div>
      </div>

      <div className="flex gap-2">
        <Button variant="outline" onClick={() => setRefined(null)}>重新炼化</Button>
        <Link to="/souls/$name" params={{ name: encodeURIComponent(profile.name) }} className="flex-1">
          <Button className="w-full" data-testid="view-soul-btn">
            <Check className="h-4 w-4 mr-1" /> 查看角色详情
          </Button>
        </Link>
      </div>
    </div>
  );
}
