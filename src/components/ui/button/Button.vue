<script setup lang="ts">
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-bamboo/40 disabled:pointer-events-none disabled:opacity-50 select-none",
  {
    variants: {
      variant: {
        default:
          "bg-bamboo text-cloud shadow-sm hover:bg-bamboo-light active:bg-bamboo",
        secondary:
          "bg-paper-warm text-ink-soft shadow-sm hover:bg-paper-deep hover:text-ink active:bg-paper-deep",
        ghost:
          "bg-transparent text-ink-soft hover:bg-paper-deep hover:text-ink active:bg-paper-deep",
        outline:
          "border border-ink-ghost bg-transparent text-ink-soft hover:bg-paper-warm hover:text-ink active:bg-paper-deep",
        danger:
          "bg-danger-bg text-red-600 shadow-sm hover:bg-red-100 dark:hover:bg-red-900/30 active:bg-red-200 dark:active:bg-red-900/50",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-8",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

type ButtonVariants = VariantProps<typeof buttonVariants>;

interface Props {
  variant?: ButtonVariants["variant"];
  size?: ButtonVariants["size"];
  class?: string;
}

const props = defineProps<Props>();
</script>

<template>
  <button
    :class="cn(buttonVariants({ variant: props.variant, size: props.size }), props.class)"
  >
    <slot />
  </button>
</template>
