import { serve } from "bun";
import { join } from "path";

const DIST = join(import.meta.dir, "dist");
const PORT = 1420;

serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    let path = url.pathname;
    if (path === "/") path = "/index.html";

    console.log(`[${new Date().toISOString()}] ${req.method} ${path}`);

    const file = Bun.file(join(DIST, path));
    if (await file.exists()) {
      return new Response(file);
    }

    // SPA fallback
    const index = Bun.file(join(DIST, "index.html"));
    if (await index.exists()) {
      console.log(`[${new Date().toISOString()}] SPA fallback -> /index.html`);
      return new Response(index);
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`Bun dev server: http://127.0.0.1:${PORT}`);
