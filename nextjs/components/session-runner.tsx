"use client";

import { useWebSocket, type ProcessStep, type SoulMessage } from "@/hooks/use-websocket";
import { SoulResponseCard } from "@/components/soul-response-card";
import { ConferenceView } from "@/components/conference-view";
import { DebateView } from "@/components/debate-view";
import { RelayView } from "@/components/relay-view";
import { SingleView } from "@/components/single-view";
import { Brain, Loader2, AlertTriangle, Key, ArrowRight, CheckCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Link } from "@tanstack/react-router";

import FollowUpInput from "@/components/follow-up-input";
import { useState, useEffect } from "react";
import { useDomain } from "@/contexts/domain-context";

interface MatchedSoulInfo {
  name: string;
  field: string;
  ismism_code: string;
}

interface SessionRunnerProps {
  sessionId: string;
  mode: string;
  matchedSouls?: MatchedSoulInfo[];
  taskTitle?: string;
  onDone?: () => void;
  sessionDone?: boolean;
}

import { modeLabel } from "@/config/possession-modes";

function ConnectingView({ agentNoun }: { agentNoun: string }) {
  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-4 text-muted-foreground">
      <Loader2 className="h-8 w-8 animate-spin text-primary" />
      <div className="text-center space-y-1">
        <p className="text-lg font-medium">正在建立连接...</p>
        <p className="text-sm">连接到{agentNoun}服务，准备开始</p>
      </div>
    </div>
  );
}

function SoulCardGrid({
  souls,
  messages,
  arrivedSet,
}: {
  souls: MatchedSoulInfo[];
  messages: Record<string, SoulMessage>;
  arrivedSet: Set<string>;
}) {
  return (
    <div className="grid gap-3 grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 p-4">
      {souls.map((soul) => {
        const soulMsg = messages[soul.name];
        const hasArrived = arrivedSet.has(soul.name);
        const hasContent = (soulMsg?.content || "").length > 0;

        return (
          <SoulResponseCard
            key={soul.name}
            name={soul.name}
            content={soulMsg?.content || ""}
            ismismCode={soul.ismism_code}
            isStreaming={
              hasArrived && !hasContent
                ? true
                : (soulMsg?.isStreaming || false)
            }
          />
        );
      })}
    </div>
  );
}

function SoulCardGridFromMessages({ messages }: { messages: Record<string, SoulMessage> }) {
  const arrived = new Set(
    Object.entries(messages)
      .filter(([, m]) => m.content.length > 0 || !m.isStreaming)
      .map(([name]) => name)
  );
  const souls: MatchedSoulInfo[] = Object.entries(messages).map(([name, m]) => ({
    name,
    field: m.content ? "已回应" : "",
    ismism_code: m.ismismCode || "",
  }));

  return (
    <div className="grid gap-3 grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 p-4">
      {souls.map((soul) => {
        const soulMsg = messages[soul.name];
        const hasContent = (soulMsg?.content || "").length > 0;
        return (
          <SoulResponseCard
            key={soul.name}
            name={soul.name}
            content={soulMsg?.content || ""}
            ismismCode={soul.ismism_code}
            isStreaming={soulMsg?.isStreaming || false}
          />
        );
      })}
    </div>
  );
}

function WaitingSoulsView({
  matchedSouls,
  processSteps,
  messages,
  agentNoun,
}: {
  matchedSouls?: MatchedSoulInfo[];
  processSteps?: ProcessStep[];
  messages: Record<string, SoulMessage>;
  agentNoun: string;
}) {
  const steps = processSteps || [];
  const arrivedSouls = steps
    .filter((s) => s.event === "SoulStarted" || s.event === "SoulDone")
    .map((s) => s.soulName || "");

  const classified = steps.find((s) => s.event === "EntryClassified");
  const parsedFromClassified: string[] = [];
  if (classified) {
    const match = classified.message.match(/匹配.[：:]\s*(.+)/);
    if (match) {
      match[1].split(/[,，、]\s*/).forEach((n) => { if (n.trim()) parsedFromClassified.push(n.trim()); });
    }
  }

  const allSoulNames = Array.from(new Set([...arrivedSouls, ...parsedFromClassified]));
  const dynamicSouls: MatchedSoulInfo[] = allSoulNames.map((name) => ({
    name, field: arrivedSouls.includes(name) ? "已召唤" : "等待召唤…", ismism_code: "",
  }));
  const souls = matchedSouls && matchedSouls.length > 0 ? matchedSouls : (dynamicSouls.length > 0 ? dynamicSouls : null);
  const arrivedSet = new Set(arrivedSouls);

  if (!souls) {
    return (
      <div className="flex flex-col items-center justify-center flex-1 gap-4 text-muted-foreground">
        <Brain className="h-8 w-8 text-primary animate-pulse" />
        <div className="text-center space-y-2">
          <p className="text-lg font-medium">{agentNoun}正在思考...</p>
          <p className="text-sm">首次调用可能需要 5-10 秒，请耐心等待</p>
        </div>
      </div>
    );
  }

  return <SoulCardGrid souls={souls} messages={messages} arrivedSet={arrivedSet} />;
}

function RequireApiKeyView() {
  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-4 p-8">
      <div className="max-w-md text-center space-y-4">
        <Key className="h-10 w-10 text-yellow-500 mx-auto" />
        <h3 className="text-lg font-semibold">角色无法回应</h3>
        <p className="text-sm text-muted-foreground">需要配置 LLM API Key 才能驱动角色</p>
        <div className="text-xs text-left space-y-1 bg-muted rounded-lg p-3 font-mono">
          <p>export ANTHROPIC_API_KEY=sk-ant-...</p>
          <p>export OPENAI_API_KEY=sk-...</p>
          <p>export DEEPSEEK_API_KEY=sk-...</p>
        </div>
        <p className="text-xs text-muted-foreground">设置后重启 API 服务即可</p>
      </div>
    </div>
  );
}

function ErrorView({ error, onReconnect }: { error: string; onReconnect: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-4 p-8">
      <AlertTriangle className="h-10 w-10 text-red-500" />
      <h3 className="text-lg font-semibold">连接错误</h3>
      <p className="text-sm text-muted-foreground text-center max-w-md">{error}</p>
      <button onClick={onReconnect} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground" data-testid="reconnect-btn">
        重新连接
      </button>
    </div>
  );
}

export function SessionRunner({ sessionId, mode, matchedSouls, taskTitle, onDone, sessionDone }: SessionRunnerProps) {
  const { messages, synthesis, status, error, processSteps, cost, collisions, toolCalls, soulRecommendations, reconnect } =
    useWebSocket(sessionId);
  const { agentNoun, synthesisVerb } = useDomain();

  const hasMessages = Object.keys(messages).length > 0;
  const streamingSouls = Object.entries(messages)
    .filter(([, m]) => m.isStreaming)
    .map(([name]) => name);

  const lastStep = processSteps[processSteps.length - 1];
  const classifiedStep = processSteps.find((s) => s.event === "EntryClassified");
  const synthesisStarted = processSteps.some((s) => s.event === "SynthesisStarted");
  let progressText = "";
  if (status === "streaming") {
    if (streamingSouls.length > 0) {
      progressText = `${streamingSouls.join("、")} 生成中…`;
    } else if (synthesis && synthesis.length > 0) {
      progressText = `${synthesisVerb}生成中…`;
    } else if (synthesisStarted) {
      progressText = `即将生成${synthesisVerb}…`;
    } else if (classifiedStep) {
      const soulsInMsg = classifiedStep.message.match(/匹配.[：:]\s*(.+)/);
      const soulCount = soulsInMsg ? soulsInMsg[1].split(/[,，、]/).length : 0;
      progressText = `已匹配 ${soulCount} ${agentNoun}，等待回应…`;
    } else if (lastStep) {
      progressText = lastStep.message;
    } else {
      progressText = "初始化中…";
    }
  } else if (status === "done" && hasMessages) {
    progressText = "会话完成";
  } else if (status === "error") {
    progressText = "连接中断";
  }

  useEffect(() => {
    if (status === "done" && hasMessages && onDone && !sessionDone) {
      onDone();
    }
  }, [status, hasMessages, onDone, sessionDone]);

  return (
    <div className="flex flex-col flex-1" data-testid="session-runner">
      {progressText && (
        <div className={cn(
          "px-4 py-2 text-sm border-b transition-colors flex items-center justify-between",
          (status === "done" || (status === "streaming" && synthesis)) ? "bg-green-50 dark:bg-green-950/20 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800" :
          status === "error" ? "bg-red-50 dark:bg-red-950/20 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800" :
          "bg-primary/5 text-primary border-primary/20"
        )}>
          <span className="inline-flex items-center gap-2">
            {(status === "streaming" && !synthesis) && <Loader2 className="h-3 w-3 animate-spin" />}
            {status === "done" && <CheckCircle className="h-3 w-3 text-green-500" />}
            {status === "error" && <AlertTriangle className="h-3 w-3 text-red-500" />}
            {(status === "streaming" && synthesis) && <CheckCircle className="h-3 w-3 text-green-500" />}
            <span className="font-medium">{progressText}</span>
          </span>
          {(status === "done" || (status === "streaming" && synthesis)) && (
            <Link
              to="/sessions/$id"
              params={{ id: sessionId }}
              className="text-xs text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-200 underline underline-offset-2 transition-colors"
            >
              查看会话 →
            </Link>
          )}
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {status === "connecting" && !hasMessages && (matchedSouls && matchedSouls.length > 0 ? (
          <WaitingSoulsView matchedSouls={matchedSouls} processSteps={processSteps} messages={messages} agentNoun={agentNoun} />
        ) : (
          <ConnectingView agentNoun={agentNoun} />
        ))}
        {status === "streaming" && !hasMessages && (
          <WaitingSoulsView matchedSouls={matchedSouls} processSteps={processSteps} messages={messages} agentNoun={agentNoun} />
        )}
        {status === "error" && !hasMessages && <ErrorView error={error || "未知错误"} onReconnect={reconnect} />}
        {status === "done" && error && !hasMessages && <ErrorView error={error} onReconnect={reconnect} />}
        {status === "done" && !error && !hasMessages && <RequireApiKeyView />}
        {hasMessages && (
          <div className="px-4 pb-4 space-y-4">
            {mode === "conference" && <ConferenceView messages={messages} synthesis={synthesis} collisions={collisions} toolCalls={toolCalls} />}
            {mode === "debate" && <DebateView messages={messages} />}
            {mode === "relay" && <RelayView messages={messages} />}
            {mode === "single" && <SingleView messages={messages} />}
            {mode === "learn" && (
              <div className="p-6 text-center text-muted-foreground">
                学习模式已移除
              </div>
            )}
          </div>
        )}
      </div>

      {status === "done" && hasMessages && (
        <div className="border-t p-4 space-y-6">
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            <Link
              to="/sessions/$id"
              params={{ id: sessionId }}
              className="inline-flex items-center gap-2 text-sm font-medium text-primary hover:underline"
            >
              <ArrowRight className="h-4 w-4" />
              前往完整对话
            </Link>
          </div>
          <FollowUpInput sessionId={sessionId} />
        </div>
      )}
    </div>
  );
}
