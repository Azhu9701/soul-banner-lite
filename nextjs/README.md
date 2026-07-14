# 模拟仲裁庭 — 前端

基于 **TanStack Start** (React 19 + Vite) 构建。

## 启动

```bash
pnpm dev      # 开发服务器 http://localhost:3000
pnpm build    # 构建生产版本
pnpm start    # 启动生产服务器
```

## 技术栈

- **框架**: TanStack Start 1.x + TanStack Router
- **UI**: React 19 + shadcn/ui + Tailwind CSS v4
- **构建**: Vite 7
- **包管理**: pnpm

## 目录

```
app/          # 路由（TanStack Router 扁平路由）
components/   # React 组件
contexts/     # React Context
hooks/        # 自定义 Hooks
lib/          # API 客户端、工具函数
config/       # 配置常量
public/       # 静态资源
```

## 路由

| 路由 | 文件 |
|------|------|
| `/` | `app/index.tsx` |
| `/souls` | `app/souls.tsx` |
| `/souls/$name` | `app/souls.$name.tsx` |
| `/souls/collect` | `app/souls.collect.tsx` |
| `/souls/refine` | `app/souls.refine.tsx` |
| `/possess` | `app/possess.tsx` |
| `/possess/$sessionId` | `app/possess.$sessionId.tsx` |
| `/sessions` | `app/sessions.tsx` |
| `/sessions/$id` | `app/sessions.$id.tsx` |
| `/analytics` | `app/analytics.tsx` |
| `/knowledge` | `app/knowledge.tsx` |
| `/models` | `app/models.tsx` |
| `/searxng` | `app/searxng.tsx` |
