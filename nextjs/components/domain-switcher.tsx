"use client";

import { useEffect, useState } from "react";
import { useDomain } from "@/contexts/domain-context";
import { getDomainInfo, type DomainOption } from "@/lib/api";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2 } from "lucide-react";

// profile → icon 映射（和 domain-context 保持一致）
const PROFILE_ICONS: Record<string, string> = {
  philosophy: "🧭",
  legal: "⚖️",
  labor: "🛡️",
  court: "🏛",
  business: "🏢",
};

export function DomainSwitcher() {
  const { profile, systemName, icon, switchDomain } = useDomain();
  const [options, setOptions] = useState<DomainOption[]>([]);
  const [switching, setSwitching] = useState(false);

  // 加载可用领域列表
  useEffect(() => {
    getDomainInfo()
      .then((info) => setOptions(info.available))
      .catch(() => {
        // fallback：至少显示当前领域
        setOptions([
          { profile: "court", label: "模拟仲裁庭", available: true },
          { profile: "business", label: "商业沙盘推演", available: true },
          { profile: "legal", label: "法律智囊团", available: true },
        ]);
      });
  }, []);

  const handleChange = async (value: string | null) => {
    if (!value || value === profile || switching) return;
    setSwitching(true);
    try {
      await switchDomain(value);
      // 刷新页面以让所有组件重新读取领域状态
      window.location.reload();
    } catch (e) {
      console.error("Domain switch failed:", e);
    } finally {
      setSwitching(false);
    }
  };

  return (
    <div className="flex items-center gap-2">
      <Select value={profile} onValueChange={handleChange}>
        <SelectTrigger size="sm" className="w-fit gap-1.5 font-medium" disabled={switching}>
          {switching ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <span className="text-base leading-none">{icon}</span>
          )}
          <SelectValue />
        </SelectTrigger>
        <SelectContent align="end">
          {options.map((opt) => (
            <SelectItem key={opt.profile} value={opt.profile}>
              <span className="mr-1.5">{PROFILE_ICONS[opt.profile] ?? "🌐"}</span>
              {opt.label}
              {!opt.available && (
                <span className="ml-1 text-xs text-muted-foreground">(未安装)</span>
              )}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
