import { defineStore } from "pinia";
import { ref } from "vue";

export interface ToolItem {
  id: string;
  label: string;
  category: string;
}

export const useDevToolStore = defineStore("dev-tools", () => {
  const selectedTool = ref("json-formatter");

  const tools: ToolItem[] = [
    // Data Format
    { id: "json-formatter", label: "JSON 格式化", category: "data-format" },
    { id: "yaml-formatter", label: "YAML 格式化", category: "data-format" },
    { id: "toml-formatter", label: "TOML 格式化", category: "data-format" },
    { id: "format-converter", label: "格式互转", category: "data-format" },
    { id: "base64-codec", label: "Base64 编解码", category: "data-format" },
    { id: "hex-codec", label: "Hex 编解码", category: "data-format" },
    { id: "url-codec", label: "URL 编解码", category: "data-format" },
    { id: "html-entity-codec", label: "HTML 实体编解码", category: "data-format" },
    // Dev Tools
    { id: "jwt-decoder", label: "JWT 解码器", category: "dev-tools" },
    { id: "cron-expression", label: "Cron 表达式", category: "dev-tools" },
    { id: "uuid-generator", label: "UUID 生成器", category: "dev-tools" },
    { id: "timestamp-converter", label: "时间戳转换", category: "dev-tools" },
    { id: "hash-generator", label: "Hash 生成器", category: "dev-tools" },
    // Code Tools
    { id: "text-diff", label: "文本 Diff", category: "code-tools" },
    { id: "naming-converter", label: "命名格式转换", category: "code-tools" },
    { id: "sql-formatter", label: "SQL 格式化", category: "code-tools" },
    { id: "regex-tester", label: "正则测试器", category: "code-tools" },
  ];

  const categories = [
    { id: "data-format", label: "数据格式" },
    { id: "dev-tools", label: "开发调试" },
    { id: "code-tools", label: "代码辅助" },
  ] as const;

  function selectTool(id: string) {
    selectedTool.value = id;
  }

  function getToolsByCategory(categoryId: string) {
    return tools.filter((t) => t.category === categoryId);
  }

  return { selectedTool, tools, categories, selectTool, getToolsByCategory };
});
