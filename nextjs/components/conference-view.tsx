"use client";

import { useState, useMemo, useCallback } from "react";
import type { SoulMessage, CollisionEvent, ToolCallEvent } from "@/hooks/use-websocket";
import { SoulPanel } from "@/components/soul-panel";
import { SynthesisSection } from "@/components/synthesis-section";
import { CollisionNotification } from "@/components/collision-notification";
import { ToolCallList } from "@/components/tool-call-indicator";
import { ArticleModal } from "@/components/article-modal";
import { useDomain } from "@/contexts/domain-context";

/** 法庭角色名称映射——在 court 领域下为5个法庭角色显示角色标签 */
const COURT_ROLE_LABELS: Record<string, string> = {
  "仲裁法官": "⚖️ 审判长",
  "原告律师": "📋 原告代理人",
  "被告律师": "🛡 被告代理人",
  "专家证人": "🔬 专家证人",
  "劳动者之声": "👤 当事人",
};

/** 商业推演角色名称映射 */
const BUSINESS_ROLE_LABELS: Record<string, string> = {
  "CEO": "🎯 总经理",
  "CFO": "📊 财务总监",
  "COO": "🏭 运营总监",
  "CMO": "📈 市场总监",
  "管理咨询顾问": "💡 管理咨询顾问",
};

interface ConferenceViewProps {
  messages: Record<string, SoulMessage>;
  synthesis: string;
  collisions: CollisionEvent[];
  toolCalls: ToolCallEvent[];
}

export function ConferenceView({ messages, synthesis, collisions, toolCalls }: ConferenceViewProps) {
  const names = useMemo(() => Object.keys(messages), [messages]);
  const [focusedSoul, setFocusedSoul] = useState<string | null>(null);
  const { profile, agentNoun, agentNoun: soulLabel } = useDomain();
  const isCourt = profile === "court";

  const hasActiveCollisions = useMemo(
    () => collisions.some(c => c.to === names.find(n => messages[n].isStreaming)),
    [collisions, names, messages]
  );
  const streamingCount = useMemo(
    () => names.filter(n => messages[n].isStreaming).length,
    [names, messages]
  );

  const openModal = useCallback((name: string) => {
    setFocusedSoul(name);
  }, []);
  const closeModal = useCallback(() => {
    setFocusedSoul(null);
  }, []);

  return (
    <div data-testid="conference-view" className="flex-1 flex flex-col h-full overflow-hidden">
      {/* 顶部信息栏 */}
      <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/20">
        <div className="flex items-center gap-4">
          <span className="text-sm font-medium">{isCourt ? "庭审" : "合议模式"}</span>
          <span className="text-xs text-muted-foreground">{names.length} {soulLabel}参与</span>
          {hasActiveCollisions && (
            <span className="text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full animate-pulse">
              交叉追问进行中
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {streamingCount > 0 && (
            <span className="text-xs text-muted-foreground">
              {streamingCount} {soulLabel}正在回应
            </span>
          )}
        </div>
      </div>

      {/* 工具调用通知 */}
      {toolCalls.length > 0 && (
        <div className="px-4 pt-3">
          <ToolCallList toolCalls={toolCalls} />
        </div>
      )}

      {/* 角色面板区 */}
      {isCourt ? (
        <CourtLayout names={names} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
      ) : (
        <div className="flex-1 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 p-3 overflow-hidden">
          {names.map((name) => (
            <SoulPanelButton key={name} name={name} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
          ))}
        </div>
      )}

      {/* 碰撞通知栏 - 实时弹出 */}
      {collisions.length > 0 && (
        <CollisionNotification collisions={collisions} />
      )}

      {/* 辩证综合 — 与历史详情页统一卡片样式 */}
      {synthesis && (
        <SynthesisSection messages={[{ id: "synthesis", content: synthesis, created_at: new Date().toISOString() }]} />
      )}

      {/* 沉浸阅读 modal — 点击卡片后弹出 */}
      {focusedSoul && messages[focusedSoul] && (
        <ArticleModal
          isOpen={!!focusedSoul}
          onClose={closeModal}
          title={focusedSoul}
          ismismCode={messages[focusedSoul].ismismCode || ""}
          content={messages[focusedSoul].content}
        />
      )}
    </div>
  );
}

/** 法庭布局——审判长上位，原告/被告分列两侧，证人居中 */
function CourtLayout({
  names, messages, collisions, toolCalls, openModal,
}: {
  names: string[];
  messages: Record<string, SoulMessage>;
  collisions: CollisionEvent[];
  toolCalls: ToolCallEvent[];
  openModal: (name: string) => void;
}) {
  const judge = names.find(n => n === "仲裁法官");
  const plaintiff = names.find(n => n === "原告律师");
  const defendant = names.find(n => n === "被告律师");
  const expert = names.find(n => n === "专家证人");
  const worker = names.find(n => n === "劳动者之声");
  const others = names.filter(n => !["仲裁法官", "原告律师", "被告律师", "专家证人", "劳动者之声"].includes(n));

  return (
    <div className="flex-1 flex flex-col gap-2 p-3 overflow-hidden">
      {/* 法官席——顶部 */}
      {judge && (
        <div className="flex justify-center">
          <div className="w-full max-w-2xl">
            <SoulPanelButton name={judge} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
          </div>
        </div>
      )}

      {/* 中间区域：原告侧 vs 被告侧 */}
      <div className="flex-1 grid grid-cols-2 gap-3 min-h-0">
        {/* 原告侧 */}
        <div className="flex flex-col gap-2 overflow-hidden">
          <div className="text-xs text-muted-foreground font-medium px-1">原告方</div>
          {worker && (
            <div className="flex-1 min-h-0">
              <SoulPanelButton name={worker} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
            </div>
          )}
          {plaintiff && (
            <div className="flex-1 min-h-0">
              <SoulPanelButton name={plaintiff} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
            </div>
          )}
        </div>

        {/* 被告侧 */}
        <div className="flex flex-col gap-2 overflow-hidden">
          <div className="text-xs text-muted-foreground font-medium px-1">被告方</div>
          {defendant && (
            <div className="flex-1 min-h-0">
              <SoulPanelButton name={defendant} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
            </div>
          )}
        </div>
      </div>

      {/* 专家证人——底部 */}
      {expert && (
        <div className="flex justify-center">
          <div className="w-full max-w-xl">
            <SoulPanelButton name={expert} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
          </div>
        </div>
      )}

      {/* 其他角色（如有） */}
      {others.length > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
          {others.map(name => (
            <SoulPanelButton key={name} name={name} messages={messages} collisions={collisions} toolCalls={toolCalls} openModal={openModal} />
          ))}
        </div>
      )}
    </div>
  );
}

/** 角色面板按钮——复用逻辑 */
function SoulPanelButton({
  name, messages, collisions, toolCalls, openModal,
}: {
  name: string;
  messages: Record<string, SoulMessage>;
  collisions: CollisionEvent[];
  toolCalls: ToolCallEvent[];
  openModal: (name: string) => void;
}) {
  const roleLabel = COURT_ROLE_LABELS[name] || BUSINESS_ROLE_LABELS[name];
  const soulToolCalls = toolCalls.filter(tc => tc.soulName === name);
  return (
    <button
      type="button"
      className="w-full h-full text-left transition-all duration-300"
      onClick={() => openModal(name)}
      onKeyDown={(e) => {
        if (e.nativeEvent.isComposing || e.keyCode === 229) return;
        if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openModal(name); }
      }}
      aria-label={`${name} 面板 — 点击展开沉浸阅读`}
    >
      <SoulPanel
        name={name}
        content={messages[name].content}
        isStreaming={messages[name].isStreaming}
        error={messages[name].error}
        hasCollision={collisions.some(c => c.to === name || c.from === name)}
        ismismCode={messages[name].ismismCode || ""}
        isExpanded={false}
        roleLabel={roleLabel}
        toolCalls={soulToolCalls}
      />
    </button>
  );
}
