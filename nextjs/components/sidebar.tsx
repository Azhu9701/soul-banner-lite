"use client";

import { useLocation } from "@tanstack/react-router";
import { useSidebar } from "@/contexts/sidebar-context";
import { cn } from "@/lib/utils";
import { SidebarLogo } from "@/components/sidebar-logo";
import { SidebarNav } from "@/components/sidebar-nav";
import { SidebarSessions } from "@/components/sidebar-sessions";
import { SidebarFooter } from "@/components/sidebar-footer";
import { Button } from "@/components/ui/button";
import { ChevronRight, GripVertical } from "lucide-react";
import { useRef, useEffect } from "react";

export function Sidebar() {
  const { collapsed, toggle, ready, width, setWidth } = useSidebar();
  const pathname = useLocation().pathname;
  const isDragging = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);
  const moveRef = useRef<((e: MouseEvent) => void) | null>(null);
  const upRef = useRef<(() => void) | null>(null);

  const isCollapsed = ready ? collapsed : false;

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    isDragging.current = true;
    startX.current = e.clientX;
    startWidth.current = width;

    const onMove = (ev: MouseEvent) => {
      if (!isDragging.current) return;
      setWidth(startWidth.current + (ev.clientX - startX.current));
    };

    const onUp = () => {
      isDragging.current = false;
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      moveRef.current = null;
      upRef.current = null;
    };

    moveRef.current = onMove;
    upRef.current = onUp;
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };

  useEffect(() => {
    return () => {
      if (moveRef.current) document.removeEventListener("mousemove", moveRef.current);
      if (upRef.current) document.removeEventListener("mouseup", upRef.current);
    };
  }, []);

  return (
    <>
      {/* Sidebar */}
      <aside
        data-testid="app-sidebar"
        className={cn(
          "relative flex flex-col border-r bg-background transition-all duration-200 shrink-0 overflow-hidden",
          isCollapsed ? "w-0 border-r-0" : ""
        )}
        style={{
          width: isCollapsed ? 0 : width,
        }}
      >
        <SidebarLogo />
        <SidebarNav currentPath={pathname} />
        <SidebarSessions />
        <SidebarFooter />
      </aside>

      {/* Resizer */}
      {ready && !isCollapsed && (
        <div
          className="shrink-0 w-1 hover:w-1.5 cursor-col-resize bg-transparent hover:bg-border transition-all duration-150 flex items-center justify-center group"
          onMouseDown={handleMouseDown}
          onKeyDown={(e) => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); setWidth(width + (e.key === "ArrowLeft" ? -10 : 10)); } }}
          title="拖拽调整宽度"
          role="slider"
          aria-label="调整侧边栏宽度"
          aria-valuemin={100}
          aria-valuemax={400}
          aria-valuenow={width}
          tabIndex={0}
        >
          <div className="opacity-0 group-hover:opacity-100 transition-opacity">
            <GripVertical className="h-4 w-4 text-muted-foreground" />
          </div>
        </div>
      )}

      {/* Floating expand button when collapsed */}
      {ready && isCollapsed && (
        <div className="relative shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="absolute left-0 top-4 z-30 h-8 w-6 rounded-l-none border border-l-0 bg-background hover:bg-muted"
            onClick={toggle}
            data-testid="sidebar-expand-btn"
            aria-label="展开侧边栏"
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      )}
    </>
  );
}
