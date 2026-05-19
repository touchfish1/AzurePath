import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useCommandPaletteStore } from "@/stores/commandPalette";

describe("commandPalette store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("defaults to closed with empty query and commands", () => {
    const store = useCommandPaletteStore();
    expect(store.isOpen).toBe(false);
    expect(store.query).toBe("");
    expect(store.commands).toEqual([]);
    expect(store.filteredCommands).toEqual([]);
  });

  describe("toggle", () => {
    it("opens when closed", () => {
      const store = useCommandPaletteStore();
      store.toggle();
      expect(store.isOpen).toBe(true);
    });

    it("closes when open", () => {
      const store = useCommandPaletteStore();
      store.isOpen = true;
      store.toggle();
      expect(store.isOpen).toBe(false);
    });
  });

  describe("open", () => {
    it("sets isOpen to true", () => {
      const store = useCommandPaletteStore();
      store.open();
      expect(store.isOpen).toBe(true);
    });

    it("clears query", () => {
      const store = useCommandPaletteStore();
      store.query = "test";
      store.open();
      expect(store.query).toBe("");
    });

    it("works when already open", () => {
      const store = useCommandPaletteStore();
      store.isOpen = true;
      store.query = "test";
      store.open();
      expect(store.isOpen).toBe(true);
      expect(store.query).toBe("");
    });
  });

  describe("close", () => {
    it("sets isOpen to false", () => {
      const store = useCommandPaletteStore();
      store.isOpen = true;
      store.close();
      expect(store.isOpen).toBe(false);
    });

    it("clears query", () => {
      const store = useCommandPaletteStore();
      store.isOpen = true;
      store.query = "test";
      store.close();
      expect(store.query).toBe("");
    });

    it("works when already closed", () => {
      const store = useCommandPaletteStore();
      store.close();
      expect(store.isOpen).toBe(false);
    });
  });

  describe("registerCommands", () => {
    it("sets the commands array", () => {
      const store = useCommandPaletteStore();
      const cmds = [
        { id: "1", label: "Test", category: "action" as const, action: () => {} },
      ];
      store.registerCommands(cmds);
      expect(store.commands).toEqual(cmds);
    });

    it("replaces previous commands", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([
        { id: "1", label: "Old", category: "action" as const, action: () => {} },
      ]);
      store.registerCommands([
        { id: "2", label: "New", category: "navigation" as const, action: () => {} },
      ]);
      expect(store.commands).toHaveLength(1);
      expect(store.commands[0].id).toBe("2");
    });
  });

  describe("addCommands", () => {
    it("appends to existing commands", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([
        { id: "1", label: "First", category: "action" as const, action: () => {} },
      ]);
      store.addCommands([
        { id: "2", label: "Second", category: "navigation" as const, action: () => {} },
      ]);
      expect(store.commands).toHaveLength(2);
    });

    it("works when commands is empty", () => {
      const store = useCommandPaletteStore();
      store.addCommands([
        { id: "1", label: "First", category: "action" as const, action: () => {} },
      ]);
      expect(store.commands).toHaveLength(1);
    });
  });

  describe("filteredCommands", () => {
    const sampleCmd = {
      id: "1",
      label: "Ping Test",
      description: "Send ICMP echo requests",
      keywords: ["icmp", "network", "latency"],
      category: "tool" as const,
      action: () => {},
    };

    it("returns all commands (up to 20) when query is empty", () => {
      const store = useCommandPaletteStore();
      const cmds = Array.from({ length: 25 }, (_, i) => ({
        id: `${i}`,
        label: `Command ${i}`,
        category: "action" as const,
        action: () => {},
      }));
      store.registerCommands(cmds);
      expect(store.filteredCommands).toHaveLength(20);
    });

    it("returns fewer than 20 when there are not enough commands", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([
        { id: "1", label: "One", category: "action" as const, action: () => {} },
        { id: "2", label: "Two", category: "navigation" as const, action: () => {} },
      ]);
      expect(store.filteredCommands).toHaveLength(2);
    });

    it("returns empty array when no commands exist", () => {
      const store = useCommandPaletteStore();
      expect(store.filteredCommands).toEqual([]);
    });

    it("filters by label case-insensitively", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "ping";
      expect(store.filteredCommands).toHaveLength(1);
      expect(store.filteredCommands[0].id).toBe("1");
    });

    it("filters by description", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "echo";
      expect(store.filteredCommands).toHaveLength(1);
    });

    it("filters by keywords", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "icmp";
      expect(store.filteredCommands).toHaveLength(1);
    });

    it("returns empty array when no match", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "zzzznotfound";
      expect(store.filteredCommands).toEqual([]);
    });

    it("ignores queries that are only whitespace", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "   ";
      expect(store.filteredCommands).toHaveLength(1); // shows all
    });

    it("only returns matching commands, not all", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([
        sampleCmd,
        {
          id: "2",
          label: "Traceroute",
          description: "Trace network path",
          category: "tool" as const,
          action: () => {},
        },
        {
          id: "3",
          label: "DNS Lookup",
          description: "Resolve domain names",
          category: "tool" as const,
          action: () => {},
        },
      ]);
      store.query = "ping";
      expect(store.filteredCommands).toHaveLength(1);
    });

    it("resets filtered results when query changes", () => {
      const store = useCommandPaletteStore();
      store.registerCommands([sampleCmd]);
      store.query = "ping";
      expect(store.filteredCommands).toHaveLength(1);

      store.query = "zzzzz";
      expect(store.filteredCommands).toHaveLength(0);

      store.query = "";
      expect(store.filteredCommands).toHaveLength(1);
    });
  });
});
