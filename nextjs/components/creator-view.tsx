"use client";

import { useState, useMemo } from "react";
import type { SoulMessage, CollisionEvent, ToolCallEvent } from "@/hooks/use-websocket";
import { SoulPanel } from "@/components/soul-panel";
import { SynthesisSection } from "@/components/synthesis-section";
import { ToolCallList } from "@/components/tool-call-indicator";
import { FileCode, Eye, X } from "lucide-react";

const CREATOR_ROLES: Record<string, { label: string; emoji: string }> = {
  "需求分析师": { label: "需求分析", emoji: "📋" },
  "技术实现师": { label: "技术实现", emoji: "💻" },
  "体验检查员": { label: "体验检查", emoji: "🔍" },
};

interface CreatorViewProps {
  messages: Record<string, SoulMessage>;
  synthesis: string;
  collisions: CollisionEvent[];
  toolCalls: ToolCallEvent[];
}

export function CreatorView({ messages, synthesis, collisions, toolCalls }: CreatorViewProps) {
  const names = useMemo(() => Object.keys(messages), [messages]);
  const [showPreview, setShowPreview] = useState(false);
  const [previewHtml, setPreviewHtml] = useState("");

  const extractHtml = (content: string): string | null => {
    const match = content.match(/```html\n?([\s\S]*?)```/);
    if (match && match[1].trim()) return match[1].trim();
    if (content.trim().startsWith("<!DOCTYPE html") || content.trim().startsWith("<html")) {
      return content.trim();
    }
    return null;
  };

  const handlePreview = () => {
    for (const name of names) {
      const msg = messages[name];
      if (msg?.content) {
        const html = extractHtml(msg.content);
        if (html) {
          setPreviewHtml(html);
          setShowPreview(true);
          return;
        }
      }
    }
    if (synthesis) {
      const html = extractHtml(synthesis);
      if (html) {
        setPreviewHtml(html);
        setShowPreview(true);
      }
    }
  };

  return (
    <div className="space-y-4">
      <div className="grid gap-3 grid-cols-1 md:grid-cols-3">
        {names.map((name) => {
          const role = CREATOR_ROLES[name] || { label: name, emoji: "🤖" };
          const msg = messages[name];
          return (
            <SoulPanel
              key={name}
              name={name}
              content={msg?.content || ""}
              isStreaming={msg?.isStreaming || false}
              roleLabel={role.label}
            />
          );
        })}
      </div>

      {toolCalls.length > 0 && <ToolCallList toolCalls={toolCalls} />}

      {synthesis && (
        <div className="rounded-lg border bg-gradient-to-r from-pink-50 to-purple-50 dark:from-pink-950/10 dark:to-purple-950/10 p-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="text-sm font-semibold text-pink-600 dark:text-pink-400">
              ✨ 创作方案
            </span>
          </div>
          <SynthesisSection messages={[{ id: "synthesis", content: synthesis, created_at: new Date().toISOString() }]} />
        </div>
      )}

      {!showPreview && names.some(n => {
        const msg = messages[n];
        return msg?.content && extractHtml(msg.content);
      }) && (
        <div className="flex justify-center">
          <button
            onClick={handlePreview}
            className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-pink-500 hover:bg-pink-600 text-white font-medium shadow-lg transition-all hover:shadow-xl"
          >
            <Eye className="h-5 w-5" />
            预览效果
          </button>
        </div>
      )}

      {showPreview && (
        <div className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4">
          <div className="bg-white dark:bg-zinc-900 rounded-xl w-full max-w-5xl h-[85vh] flex flex-col shadow-2xl">
            <div className="flex items-center justify-between px-4 py-3 border-b">
              <h3 className="font-semibold flex items-center gap-2">
                <FileCode className="h-4 w-4" />
                实时预览
              </h3>
              <button
                onClick={() => setShowPreview(false)}
                className="p-1.5 rounded-lg hover:bg-muted transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            <iframe
              srcDoc={previewHtml}
              className="flex-1 w-full rounded-b-xl"
              title="创作预览"
              sandbox="allow-scripts"
            />
          </div>
        </div>
      )}
    </div>
  );
}
