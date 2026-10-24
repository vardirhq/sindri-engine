/** The page build, whose own sources are plain JavaScript. */
export const EXAMPLES: [string, string][];
export function build(outDir: string, options?: { vendor?: boolean }): Promise<string>;
