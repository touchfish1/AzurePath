import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useToastStore } from "@/stores/toast";

describe("toast store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  it("starts with empty toasts", () => {
    const store = useToastStore();
    expect(store.toasts).toEqual([]);
  });

  it("adds a toast with the given type and message", () => {
    const store = useToastStore();
    store.add("success", "操作成功");
    expect(store.toasts).toHaveLength(1);
    expect(store.toasts[0].message).toBe("操作成功");
    expect(store.toasts[0].type).toBe("success");
    expect(store.toasts[0].id).toBe(0);
  });

  it("add increments nextId", () => {
    const store = useToastStore();
    store.add("info", "first");
    store.add("error", "second");
    expect(store.toasts[0].id).toBe(0);
    expect(store.toasts[1].id).toBe(1);
  });

  it("auto-removes toast after 2500ms", () => {
    const store = useToastStore();
    store.add("info", "auto-remove");
    expect(store.toasts).toHaveLength(1);

    vi.advanceTimersByTime(2500);
    expect(store.toasts).toHaveLength(0);
  });

  it("remove deletes a toast by id", () => {
    const store = useToastStore();
    store.add("success", "msg1");
    store.add("error", "msg2");
    expect(store.toasts).toHaveLength(2);

    store.remove(0);
    expect(store.toasts).toHaveLength(1);
    expect(store.toasts[0].message).toBe("msg2");
  });

  it("success helper creates a success toast", () => {
    const store = useToastStore();
    store.success("成功");
    expect(store.toasts[0].type).toBe("success");
  });

  it("error helper creates an error toast", () => {
    const store = useToastStore();
    store.error("错误");
    expect(store.toasts[0].type).toBe("error");
  });

  it("info helper creates an info toast", () => {
    const store = useToastStore();
    store.info("信息");
    expect(store.toasts[0].type).toBe("info");
  });
});
