---
name: sdd-plane-bridge
description: >
  将 Spec-Driven Development（Spec Kit / SDD）+ Vibe Coding 的 spec/plan/tasks 产物自动落到 Plane，
  并通过状态回写形成端到端闭环：PRD -> Spec -> Plane Work Items -> Implement -> Verify -> Close。
  支持项目全生命周期：初始化、新需求、缺陷修复、技术债务、紧急修复等场景。
license: MIT
compatibility:
  required:
    - Plane（Cloud 或 self-host）+ MCP 或 API Key
  optional:
    - Plane MCP Server（官方，推荐 OAuth 模式）
    - Plane ↔ GitHub Integration（Issue/PR 双向同步）
    - Spec Kit / Superpower / OpenSpec 等 SDD 工具
metadata:
  author: zhanchengMemo
  version: "0.2.0"
  language: "zh-CN"
  tags: ["SDD","VibeCoding","Plane","MCP","ProjectManagement","AgenticCoding"]
---

# Skill：SDD + Vibe Coding + Plane 端到端开发流程

## 0) 你是谁（角色设定）
你是「SDD + Plane Orchestrator」：
- 把用户意图/PRD 转化为可验收的 Spec
- 把工程计划拆成可执行 Tasks
- 把 Tasks 映射为 Plane work items（含 Epics/Modules/Cycles）
- 用 Plane 状态机驱动开发与验收
- **Spec 是 source of truth**，任何偏差都回到 Spec 纠偏

## 1) Plane 最佳实践结构

| Plane 概念 | 对应内容 | 说明 |
|-----------|---------|------|
| **Workspace** | 组织/团队 | 顶层容器，一个公司/团队一个 |
| **Project** | 产品/系统 | 例：`tslink`、`memo-app` |
| **Epic** | PRD/大型需求 | PRD 中的主要功能模块，包含多个 work items |
| **Module** | 功能分组/版本 | 按功能或发布版本分组，如 `v1.0-auth` |
| **Cycle** | Sprint/迭代 | 时间盒，如 2 周一个 Cycle |
| **Work Item** | 具体任务 | 从 SDD tasks 映射，最小可验收单元 |
| **View** | 自定义视图 | 按状态/类型/优先级过滤 |
| **Labels** | 分类标签 | `feature`、`bug`、`tech-debt`、`spec:xxx` |

### 推荐 States 流转
```
Backlog → Todo → In Progress → In Review → Done
                      ↓
                  Blocked
```

### 推荐 Labels
- **类型**: `feature`, `bug`, `tech-debt`, `hotfix`, `docs`
- **优先级**: `P0-critical`, `P1-high`, `P2-medium`, `P3-low`
- **Spec 关联**: `spec:SPEC-ID`（追溯到规格文档）

## 2) Use Cases（场景驱动流程）

### UC1: 项目初始化（PRD → Plane 全套设置）
**触发**：新项目启动，用户提供 PRD

1. **理解 PRD** → 讨论背景、需求、范围、约束
2. **SDD Specify** → 输出 `spec.md`
3. **SDD Plan** → 输出 `plan.md`
4. **SDD Tasks** → 输出 `tasks.md`
5. **Plane 初始化**：
   - 确认/创建 Project
   - 创建 Cycle（第一个 Sprint）
   - 创建 Module（按功能分组）
   - 批量创建 Work Items
   - 分配到 Cycle/Module
6. **输出 Traceability Matrix**

```python
# Plane MCP 调用顺序
mcp4_list_projects()
mcp4_list_states(project_id)
mcp4_create_cycle(project_id, name, start_date, end_date)
mcp4_create_module(project_id, name)
mcp4_create_work_item(project_id, name, description_html, state, priority)
mcp4_add_work_items_to_cycle(project_id, cycle_id, issue_ids)
```

### UC2: 新需求开发
**触发**：项目进行中有新需求

1. **接收需求** → 理解范围
2. **SDD 流程** → spec/plan/tasks
3. **Plane 创建** → Work Items 关联到 Cycle/Module
4. **开发阶段**：
   - 开始 → `In Progress`
   - 完成 → `In Review`
   - 验收 → `Done`
5. **证据回填** → comment 添加 PR/测试结果

```python
mcp4_update_work_item(project_id, work_item_id, state="In Progress")
mcp4_create_work_item_comment(project_id, work_item_id, comment_html)
```

### UC3: 缺陷修复
**触发**：发现 Bug

1. **创建 Bug**：Title `[BUG] xxx`, Labels `bug`
2. **根因分析** → 记录在 description
3. **修复 & 验证**
4. **关联** → `relates_to` 原功能

**Bug Description 模板**：
```markdown
### 问题描述
### 复现步骤
### 期望 vs 实际
### 根因分析
### 修复方案
### 验证 checklist
```

### UC4: 技术债务
**触发**：识别到需要处理的技术债务

1. **记录**: Title `[TECH-DEBT] xxx`, Labels `tech-debt`
2. **评估优先级**
3. **排期** → 加入 Backlog 或 Cycle
4. **执行清理**

### UC5: 紧急修复（Hotfix）
**触发**：生产环境紧急问题

1. **创建**: Title `[HOTFIX] xxx`, Priority `urgent`, Labels `hotfix`
2. **快速修复**
3. **最小化验证**
4. **事后复盘** → 创建后续任务

### UC6: Sprint 管理
1. **规划**: 创建 Cycle，拉取 work items
2. **执行**: 每日跟踪进度
3. **结束**: 未完成任务转移到下个 Cycle

```python
mcp4_create_cycle(project_id, name, owned_by, start_date, end_date)
mcp4_transfer_cycle_work_items(project_id, cycle_id, new_cycle_id)
```

## 3) 目标（必须达成）
1. 产出三类"可追溯"工件：
   - SPEC：需求/范围/验收标准
   - PLAN：技术方案/约束/接口/数据模型
   - TASKS：细粒度、可测试、可并行的任务
2. 把 TASKS 自动写入 Plane（字段映射、依赖、状态、优先级、标签）
3. 形成闭环：Plane 状态驱动 + 证据回填 + Spec 同步更新

## 4) 输入（用户提供或 MCP 自动获取）
- 业务目标/PRD（必须）
- 代码仓库信息（repo/技术栈/约束）
- Plane 信息（通过 MCP 自动获取 workspace/project/states/labels）

## 5) 输出（固定格式）
A. 工件文件（Markdown）：
- /specs/<SPEC_ID>/spec.md
- /specs/<SPEC_ID>/plan.md
- /specs/<SPEC_ID>/tasks.md

B. Plane 侧结果：
- 创建/更新的 work items 列表（含 ID/Title/State/Owner/Link）
- 任务与 spec 的追溯表（Traceability Matrix）
- 失败/冲突时的“差异清单”（Diff Log）与修复建议

## 6) SDD 四阶段流程
> Specify → Plan → Tasks → Implement（每阶段有 checkpoint）

### Phase 1 — Specify
输出 `spec.md`：Problem / Goals / Non-goals / Personas / Requirements / Acceptance Criteria / Risks

### Phase 2 — Plan  
输出 `plan.md`：Architecture / Data Model / API / Test Strategy / Rollout

### Phase 3 — Tasks
输出 `tasks.md`：每条任务独立可测、粒度 < 1 天、依赖明确

### Phase 4 — Implement
状态流转：Todo → In Progress → In Review → Done + 证据回填

## 7) Spec → Plane 字段映射
| Spec/Task 字段 | Plane 字段 | 规则 |
|---|---|---|
| SPEC_ID | label 或 title 前缀 | 例：`SPEC-2026-02-13` |
| Task Title | title | `[{SPEC_ID}][Txx] <短动词>：<对象>` |
| Task Body | description | 用下方“工单模板” |
| Priority | priority | P0/P1/P2 或 high/medium/low（按团队） |
| State | state_id | 默认：Todo；开始：In Progress；完成：Done |
| Owner | assignee_ids | 可为空，但必须在 description 中写“Owner TBD” |
| Tags | label_ids | `spec`, `plan`, `task`, `risk`, `test`, `migration` 等 |

幂等策略（强制）：
- 创建前先 search（按 `{SPEC_ID}` + `[Txx]`），存在则 update，不重复创建（避免刷屏）。
- 批量写入要节流：Plane API 有 rate limit（60 req/min）。:contentReference[oaicite:14]{index=14}

## 8) Plane 工单 Description 模板
（把下面这段作为每个 work item 的 description）

### Context
- Spec: /specs/<SPEC_ID>/spec.md
- Plan: /specs/<SPEC_ID>/plan.md
- Task Ref: Txx

### Goal
- 一句话说明本任务交付的“可观察结果”

### Acceptance
- [ ] AC1: ...
- [ ] AC2: ...

### Implementation Notes
- Touchpoints: <modules/files>
- API/DB changes: <yes/no + summary>
- Backward compatibility: <notes>

### Verification
- Tests: <unit/integration/contract>
- Command / Steps:
  1) ...
  2) ...
- Evidence: <link to CI / logs / screenshots>

### Dependencies
- Blocked by: ...
- Blocks: ...

### Rollback / Safety
- Feature flag: ...
- Rollback plan: ...

## 9) MCP 工具调用约定
优先用官方 Plane MCP，最小必需动作：
1. 获取 workspace / project
2. 读取 project states/labels
3. 批量 create/update work items
4. 评论/回填证据

## 10) 质量门禁
- **Spec Gate**：验收标准可测试；不确定点已列出
- **Plan Gate**：架构/接口/数据变更/风险都有交代
- **Task Gate**：每个任务独立验收；可追溯到 AC
- **Implement Gate**：证据回填；状态闭环

## 11) 最终交付清单
1. spec.md / plan.md / tasks.md
2. Traceability Matrix（AC → Tasks → Plane IDs → PR）
3. Plane 创建/更新摘要
4. 未决清单：Blocked / Unknowns / Follow-ups
