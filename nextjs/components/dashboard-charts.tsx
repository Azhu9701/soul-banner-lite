"use client";

import { lazy } from "react";
import { SoulEffectivenessTable } from "@/components/soul-effectiveness-table";

const ModeBarChart = lazy(
  () => import("@/components/mode-bar-chart").then((mod) => ({ default: mod.ModeBarChart })),
);

export { ModeBarChart, SoulEffectivenessTable };
