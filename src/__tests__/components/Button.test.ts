import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import Button from "@/components/ui/button/Button.vue";

describe("Button", () => {
  it("renders default slot content", () => {
    const wrapper = mount(Button, { slots: { default: "Click me" } });
    expect(wrapper.text()).toBe("Click me");
  });

  it("applies default variant by default", () => {
    const wrapper = mount(Button, { slots: { default: "OK" } });
    expect(wrapper.classes()).toContain("bg-bamboo");
  });

  it("applies danger variant classes", () => {
    const wrapper = mount(Button, {
      props: { variant: "danger" },
      slots: { default: "Delete" },
    });
    expect(wrapper.classes()).toContain("bg-danger-bg");
  });

  it("applies ghost variant classes", () => {
    const wrapper = mount(Button, {
      props: { variant: "ghost" },
      slots: { default: "Cancel" },
    });
    expect(wrapper.classes()).toContain("bg-transparent");
  });

  it("applies default size by default", () => {
    const wrapper = mount(Button, { slots: { default: "OK" } });
    expect(wrapper.classes()).toContain("h-9");
  });

  it("applies lg size classes", () => {
    const wrapper = mount(Button, {
      props: { size: "lg" },
      slots: { default: "Large" },
    });
    expect(wrapper.classes()).toContain("px-8");
  });

  it("applies sm size classes", () => {
    const wrapper = mount(Button, {
      props: { size: "sm" },
      slots: { default: "Small" },
    });
    expect(wrapper.classes()).toContain("px-3");
  });

  it("applies icon size classes", () => {
    const wrapper = mount(Button, {
      props: { size: "icon" },
      slots: { default: "+" },
    });
    expect(wrapper.classes()).toContain("w-9");
  });

  it("emits click event", () => {
    const wrapper = mount(Button, { slots: { default: "Go" } });
    wrapper.trigger("click");
    expect(wrapper.emitted("click")).toBeTruthy();
  });

  it("can be disabled", () => {
    const wrapper = mount(Button, {
      props: { disabled: true },
      slots: { default: "Disabled" },
    });
    expect(wrapper.attributes("disabled")).toBeDefined();
  });

  it("does not emit click when disabled", () => {
    const wrapper = mount(Button, {
      props: { disabled: true },
      slots: { default: "No" },
    });
    wrapper.trigger("click");
    expect(wrapper.emitted("click")).toBeFalsy();
  });

  it("merges custom class via props", () => {
    const wrapper = mount(Button, {
      props: { class: "my-custom-class" },
      slots: { default: "Styled" },
    });
    expect(wrapper.classes()).toContain("my-custom-class");
  });
});
