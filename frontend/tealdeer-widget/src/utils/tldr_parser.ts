/**
 * TLDR Markdown 解析器
 * 将 TLDR 格式的 Markdown 解析为结构化数据
 */

export type ParsedExample = {
  description: string;
  command: string;
};

export type ParsedPage = {
  title: string;
  description: string;
  examples: ParsedExample[];
  extras: string[];
};

/**
 * 解析 TLDR Markdown 为结构化数据
 */
export function parseTldrMarkdown(markdown: string): ParsedPage | null {
  if (!markdown.trim()) {
    return null;
  }

  const lines = markdown.split('\n');

  let title = '';
  let description = '';
  const examples: ParsedExample[] = [];
  const extras: string[] = [];

  let pendingDesc = '';
  let i = 0;

  const findNextNonEmpty = (start: number): string | null => {
    for (let j = start; j < lines.length; j += 1) {
      const candidate = lines[j];
      if (candidate.trim().length > 0) {
        return candidate;
      }
    }
    return null;
  };

  const pushExample = (command: string) => {
    examples.push({
      description: pendingDesc,
      command: command,
    });
    pendingDesc = '';
  };

  while (i < lines.length) {
    const rawLine = lines[i];
    const trimmed = rawLine.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    // 标题：# title 或 title\n=====
    if (!title && trimmed.startsWith('# ')) {
      title = trimmed.substring(2).trim();
      i += 1;
      continue;
    }
    if (!title && i + 1 < lines.length && lines[i + 1].trim().match(/^=+$/)) {
      title = trimmed;
      i += 2; // 跳过 ===== 行
      continue;
    }

    // 描述：> description
    if (trimmed.startsWith('> ')) {
      if (description) {
        description += ' ';
      }
      description += trimmed.substring(2).trim();
      i += 1;
      continue;
    }

    // 示例描述（v1）：- description
    if (trimmed.startsWith('- ')) {
      pendingDesc = trimmed.substring(2).trim();
      i += 1;
      continue;
    }

    // 命令（v1）：`command`
    if (trimmed.startsWith('`') && trimmed.endsWith('`') && trimmed.length >= 2) {
      const command = trimmed.substring(1, trimmed.length - 1);
      pushExample(command);
      i += 1;
      continue;
    }

    // 命令（v2）：缩进代码行
    if (/^\s+/.test(rawLine)) {
      pushExample(trimmed);
      i += 1;
      continue;
    }

    // 可能是 v2 示例描述（下一行是缩进命令）
    const nextLine = findNextNonEmpty(i + 1);
    if (nextLine && (/^\s+/.test(nextLine) || nextLine.trim().startsWith('`'))) {
      pendingDesc = trimmed;
      i += 1;
      continue;
    }

    // 其他内容（比如 More information）
    extras.push(trimmed);
    i += 1;
  }

  if (pendingDesc) {
    extras.push(pendingDesc);
  }

  if (!title && !description && examples.length === 0 && extras.length === 0) {
    return null;
  }

  return {
    title: title || 'Unknown',
    description: description || '',
    examples: examples,
    extras: extras,
  };
}

/**
 * 标准化命令（用于去重比对）
 */
export function normalizeCommand(cmd: string): string {
  return cmd.trim().split(/\s+/).join(' ');
}
