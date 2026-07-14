## 商业推演场景实施方案

### 架构概览
基于 Mock-Court 现有架构（领域系统 + Soul 角色系统 + 会话管理系统），开辟全新的"商业推演"场景。

### 实施步骤

**第1层：商业知识库** (`data/knowledge/business/`)
- `financial-analysis.md` — 三张报表、ROE、EVA、杜邦公式
- `strategy-frameworks.md` — 波士顿矩阵、ABC成本法
- `operations-management.md` — 产能规划、供应链管理

**第2层：5个Soul角色** (`data/souls/`)
- CEO/总经理、CFO/财务总监、COO/运营总监、CMO/市场总监、管理咨询顾问
- 格式对齐现有 souls（YAML frontmatter + markdown body）

**第3层：后端API** (`rust/api/src/routes/possess.rs`)
- 新增 `POST /possess/business` 端点
- 预选5个商业角色，动态生成 task_cards
- 复用 conference 模式

**第4层：前端UI** (`nextjs/`)
- `possession-entry.tsx`: 添加"商业推演"按钮 + `onStartBusiness()`
- `conference-view.tsx`: 添加商业角色标签映射
- `page.tsx`: 家页添加「商业推演」入口

**第5层：领域配置** (`config/domain.business.yaml`)
- 商业场景专属术语定义