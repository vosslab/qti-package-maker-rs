import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("../dist/", import.meta.url)));
const contentTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".wasm", "application/wasm"],
]);
const port = Number(process.env.PORT ?? 4173);
if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("PORT must be 1..65535");

const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url ?? "/", "http://localhost").pathname);
    if (pathname === "/") {
      response.writeHead(302, { Location: "/examples/browser/index.html" }).end();
      return;
    }
    const relativePath = pathname.slice(1);
    const path = resolve(root, relativePath);
    // ASVS 5.3.2: serve only trusted build files inside the fixed package root.
    if (!path.startsWith(root + sep) && path !== root) {
      response.writeHead(403).end("Forbidden");
      return;
    }
    const contentType = contentTypes.get(extname(path));
    if (!contentType) {
      response.writeHead(404).end("Not found");
      return;
    }
    const content = await readFile(path);
    response.writeHead(200, {
      "Content-Type": contentType,
      "X-Content-Type-Options": "nosniff",
      "Cache-Control": "no-store",
    });
    response.end(content);
  } catch {
    response.writeHead(404).end("Not found");
  }
});
server.listen(port, "127.0.0.1", () => console.log(`Browser example: http://127.0.0.1:${port}`));
