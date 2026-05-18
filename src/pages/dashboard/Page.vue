<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Radio, Route, Scan, Globe, ArrowRight } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

interface ToolCard {
  label: string;
  path: string;
  icon: object;
  description: string;
}

const tools: ToolCard[] = [
  {
    label: "Ping",
    path: "/ping",
    icon: Radio,
    description: "测试网络连通性，测量目标主机的响应延迟与丢包率",
  },
  {
    label: "Traceroute",
    path: "/traceroute",
    icon: Route,
    description: "追踪数据包到达目标所经过的路由节点与每一跳的延迟",
  },
  {
    label: "端口扫描",
    path: "/port-scan",
    icon: Scan,
    description: "扫描目标主机的开放端口，识别正在运行的服务",
  },
  {
    label: "DNS 查询",
    path: "/dns",
    icon: Globe,
    description: "查询域名解析记录，支持 A / AAAA / CNAME / MX 等多种记录类型",
  },
];
</script>

<template>
  <div class="flex h-full flex-col p-8 animate-view-fade">
    <div class="mb-8">
      <h1 class="text-2xl font-display font-bold text-ink">网络工具集</h1>
      <p class="mt-1 text-sm text-ink-faint">选择一项工具开始诊断与分析</p>
    </div>

    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2">
      <RouterLink
        v-for="tool in tools"
        :key="tool.path"
        :to="tool.path"
        class="group noise-bg rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-sm transition-all hover:border-bamboo/25 hover:shadow-md hover:-translate-y-0.5"
      >
        <div class="flex items-start justify-between">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-lg bg-bamboo/10 text-bamboo transition-colors group-hover:bg-bamboo/15"
          >
            <component :is="tool.icon" class="h-5 w-5" />
          </div>
          <ArrowRight
            class="h-4 w-4 text-ink-ghost transition-all group-hover:text-bamboo group-hover:translate-x-0.5"
          />
        </div>
        <h2 class="mt-4 text-base font-display font-semibold text-ink">
          {{ tool.label }}
        </h2>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-faint">
          {{ tool.description }}
        </p>
        <div class="mt-4">
          <Button variant="ghost" size="sm" class="group-hover:text-bamboo">
            打开工具
          </Button>
        </div>
      </RouterLink>
    </div>
  </div>
</template>
