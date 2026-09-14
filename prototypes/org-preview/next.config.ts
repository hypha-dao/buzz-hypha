import type { NextConfig } from 'next';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // Standalone app inside the Buzz pnpm workspace: keep Next from treating the
  // repo-root pnpm-lock.yaml as this project's workspace root.
  outputFileTracingRoot: path.dirname(fileURLToPath(import.meta.url)),
};

export default nextConfig;
