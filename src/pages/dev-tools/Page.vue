<script setup lang="ts">
import { computed } from "vue";
import { useDevToolStore } from "@/pages/dev-tools/stores";
import DevToolSidebar from "@/pages/dev-tools/components/sidebar/DevToolSidebar.vue";
import JsonFormatter from "@/pages/dev-tools/components/DataFormat/JsonFormatter.vue";
import YamlFormatter from "@/pages/dev-tools/components/DataFormat/YamlFormatter.vue";
import TomlFormatter from "@/pages/dev-tools/components/DataFormat/TomlFormatter.vue";
import FormatConverter from "@/pages/dev-tools/components/DataFormat/FormatConverter.vue";
import Base64Codec from "@/pages/dev-tools/components/DataFormat/Base64Codec.vue";
import HexCodec from "@/pages/dev-tools/components/DataFormat/HexCodec.vue";
import UrlCodec from "@/pages/dev-tools/components/DataFormat/UrlCodec.vue";
import HtmlEntityCodec from "@/pages/dev-tools/components/DataFormat/HtmlEntityCodec.vue";
import JwtDecoder from "@/pages/dev-tools/components/DevTools/JwtDecoder.vue";
import CronExpression from "@/pages/dev-tools/components/DevTools/CronExpression.vue";
import UuidGenerator from "@/pages/dev-tools/components/DevTools/UuidGenerator.vue";
import TimestampConverter from "@/pages/dev-tools/components/DevTools/TimestampConverter.vue";
import HashGenerator from "@/pages/dev-tools/components/DevTools/HashGenerator.vue";
import TextDiff from "@/pages/dev-tools/components/CodeTools/TextDiff.vue";
import NamingConverter from "@/pages/dev-tools/components/CodeTools/NamingConverter.vue";
import SqlFormatter from "@/pages/dev-tools/components/CodeTools/SqlFormatter.vue";
import RegexTester from "@/pages/dev-tools/components/CodeTools/RegexTester.vue";

const store = useDevToolStore();

const currentComponent = computed(() => {
  const map: Record<string, object> = {
    "json-formatter": JsonFormatter,
    "yaml-formatter": YamlFormatter,
    "toml-formatter": TomlFormatter,
    "format-converter": FormatConverter,
    "base64-codec": Base64Codec,
    "hex-codec": HexCodec,
    "url-codec": UrlCodec,
    "html-entity-codec": HtmlEntityCodec,
    "jwt-decoder": JwtDecoder,
    "cron-expression": CronExpression,
    "uuid-generator": UuidGenerator,
    "timestamp-converter": TimestampConverter,
    "hash-generator": HashGenerator,
    "text-diff": TextDiff,
    "naming-converter": NamingConverter,
    "sql-formatter": SqlFormatter,
    "regex-tester": RegexTester,
  };
  return map[store.selectedTool] || JsonFormatter;
});
</script>

<template>
  <div class="flex h-full animate-view-fade">
    <DevToolSidebar />
    <div class="flex-1 overflow-y-auto p-6">
      <component :is="currentComponent" />
    </div>
  </div>
</template>
