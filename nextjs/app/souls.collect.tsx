import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { ArrowLeft, Search, Loader2, Sparkles, Wand2 } from "lucide-react"
import { API_BASE } from "@/lib/api"

export const Route = createFileRoute('/souls/collect')({
  component: CollectPage,
})

function CollectPage() {
  const navigate = useNavigate()
  const [name, setName] = useState("")
  const [loading, setLoading] = useState(false)
  const [rawMaterial, setRawMaterial] = useState("")
  const [collected, setCollected] = useState(false)
  const [supplement, setSupplement] = useState("")

  const onCollect = async () => {
    if (!name.trim()) return
    setLoading(true)
    try {
      const r = await fetch(`${API_BASE}/souls/collect`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      })
      const data = await r.json()
      setRawMaterial(data.raw_material)
      setCollected(true)
    } finally {
      setLoading(false)
    }
  }

  const onRefine = () => {
    const material = supplement ? `${rawMaterial}\n\n## 用户供奉\n${supplement}` : rawMaterial
    sessionStorage.setItem("refine-material", material)
    navigate({ to: "/souls/refine" })
  }

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="flex items-center gap-3">
        <Link to="/souls"><Button variant="ghost" size="icon"><ArrowLeft className="h-4 w-4" /></Button></Link>
        <div>
          <h1 className="text-2xl font-bold">收集角色</h1>
          <p className="text-sm text-muted-foreground">AI 辅助收集人物 raw 素材</p>
        </div>
      </div>

      {!collected ? (
        <div className="space-y-4">
          <div className="rounded-lg border p-6 space-y-4">
            <p className="text-sm text-muted-foreground">
              收集角色：输入人物名 → AI 从 6 维度搜索收集信息（生平/思想/方法论/代表作/影响争议/ismism 定位）
            </p>
            <div className="flex gap-2">
              <Input
                placeholder="人物名，如：张一鸣"
                value={name}
                onChange={(e) => setName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.nativeEvent.isComposing || e.keyCode === 229) return
                  if (e.key === "Enter") onCollect()
                }}
                className="flex-1"
                data-testid="collect-name-input"
              />
              <Button onClick={onCollect} disabled={loading || !name.trim()} data-testid="collect-btn">
                {loading ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Search className="h-4 w-4 mr-1" />}
                {loading ? "收集中..." : "开始收集"}
              </Button>
            </div>
            {loading && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-3 w-3 animate-spin" />
                AI 正在搜索和整理信息，可能需要 10-20 秒...
              </div>
            )}
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="rounded-lg border p-4">
            <h3 className="text-sm font-semibold mb-2">Raw 素材 — {name}</h3>
            <pre className="whitespace-pre-wrap text-sm bg-muted rounded-md p-4 max-h-96 overflow-y-auto">{rawMaterial}</pre>
          </div>

          <div className="rounded-lg border p-4 space-y-2">
            <h3 className="text-sm font-semibold">供奉素材（可选）</h3>
            <p className="text-xs text-muted-foreground">你对此人物的了解，将补充进 raw 素材一起炼化</p>
            <Textarea
              placeholder="补充信息、个人观察..."
              value={supplement}
              onChange={(e) => setSupplement(e.target.value)}
              rows={4}
            />
          </div>

          <div className="flex gap-2">
            <Button variant="outline" onClick={() => { setCollected(false); setRawMaterial("") }}>重新收集</Button>
            <Button onClick={onRefine} className="flex-1" data-testid="go-refine-btn">
              <Sparkles className="h-4 w-4 mr-1" /> 送交炼化
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
