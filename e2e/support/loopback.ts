import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

export type Loopback = {
  redirectUri: string;
  received: URLSearchParams[];
  nextCallback(timeoutMs?: number): Promise<URLSearchParams>;
  close(): Promise<void>;
};

export async function startLoopback(path = "/callback"): Promise<Loopback> {
  const received: URLSearchParams[] = [];
  const waiting: ((params: URLSearchParams) => void)[] = [];
  let taken = 0;

  const server: Server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    if (url.pathname === path) {
      received.push(url.searchParams);
      waiting.shift()?.(url.searchParams);
    }
    response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    response.end("<!doctype html><title>Callback</title><p>You can close this window.</p>");
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;

  return {
    redirectUri: `http://127.0.0.1:${port}${path}`,
    received,
    nextCallback(timeoutMs = 15_000) {
      const index = taken++;
      if (received[index]) return Promise.resolve(received[index]);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(
          () => reject(new Error(`no redirect reached ${path} within ${timeoutMs}ms`)),
          timeoutMs,
        );
        waiting.push((params) => {
          clearTimeout(timer);
          resolve(params);
        });
      });
    },
    close() {
      server.closeAllConnections();
      return new Promise((resolve) => server.close(() => resolve()));
    },
  };
}
