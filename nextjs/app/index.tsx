import { createFileRoute, Link } from '@tanstack/react-router'
import { Brain, History, Search, BarChart3, ArrowRight, Building2 } from "lucide-react"

const QUICK_LINKS = [
  { href: "/possess", label: "开庭", desc: "提交劳动争议案件，系统自动组建仲裁庭", icon: Brain },
  { href: "/possess?mode=conference&souls=CEO,CFO,COO,CMO,管理咨询顾问", label: "商业推演", desc: "启动商业沙盘推演，CEO/CFO/COO/CMO/顾问协同分析", icon: Building2 },
  { href: "/souls", label: "角色", desc: "仲裁庭角色档案 · 商业角色档案", icon: Search },
  { href: "/sessions", label: "庭审记录", desc: "回顾过往庭审 · 商业推演记录", icon: History },
  { href: "/analytics", label: "庭审统计", desc: "庭审效率指数 · 运行数据", icon: BarChart3 },
];

export const Route = createFileRoute('/')({
  component: Home,
})

function Home() {
  return (
    <div className="max-w-2xl mx-auto py-16 space-y-12">
      <div className="text-center space-y-4">
        <h1 className="text-4xl font-bold tracking-tight">模拟仲裁庭</h1>
        <p className="text-sm text-muted-foreground">
          法律场景 · 商业推演 · 多角色AI推演系统
        </p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {QUICK_LINKS.map((link) => {
          const Icon = link.icon;
          return (
            <Link key={link.href} to={link.href} className="group rounded-xl border p-5 hover:bg-muted/50 transition-colors">
              <div className="flex items-center gap-3 mb-2">
                <Icon className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">{link.label}</h3>
                <ArrowRight className="h-4 w-4 ml-auto opacity-0 -translate-x-2 group-hover:opacity-100 group-hover:translate-x-0 transition-all" />
              </div>
              <p className="text-sm text-muted-foreground">{link.desc}</p>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
