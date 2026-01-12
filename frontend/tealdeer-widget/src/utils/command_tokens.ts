// Tokenizer for command syntax highlighting

export type TokenType = 
  | 'command'    // First token (command name)
  | 'sudo'       // sudo keyword
  | 'option'     // -X or --flag
  | 'variable'   // {{...}}
  | 'string'     // "..." or '...'
  | 'operator'   // | && ; > <
  | 'path'       // /path/...
  | 'text';      // Default

export interface Token {
  type: TokenType;
  value: string;
}

export function tokenizeCommand(cmd: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  let isFirst = true;
  let sawSudo = false;

  while (i < cmd.length) {
    // Skip whitespace
    if (/\s/.test(cmd[i])) {
      tokens.push({ type: 'text', value: cmd[i] });
      i++;
      continue;
    }

    // Variable {{...}}
    if (cmd.slice(i, i + 2) === '{{') {
      const end = cmd.indexOf('}}', i + 2);
      if (end !== -1) {
        tokens.push({ type: 'variable', value: cmd.slice(i, end + 2) });
        i = end + 2;
        isFirst = false;
        sawSudo = false;
        continue;
      }
    }

    // String "..." or '...'
    if (cmd[i] === '"' || cmd[i] === "'") {
      const quote = cmd[i];
      let end = i + 1;
      while (end < cmd.length && cmd[end] !== quote) {
        if (cmd[end] === '\\') end++; // Skip escaped
        end++;
      }
      if (end < cmd.length) end++; // Include closing quote
      tokens.push({ type: 'string', value: cmd.slice(i, end) });
      i = end;
      isFirst = false;
      sawSudo = false;
      continue;
    }

    // Operators
    if ('|;&><'.includes(cmd[i])) {
      let op = cmd[i];
      if (i + 1 < cmd.length && '|&><'.includes(cmd[i + 1])) {
        op += cmd[i + 1];
        i++;
      }
      tokens.push({ type: 'operator', value: op });
      i++;
      isFirst = false;
      sawSudo = false;
      continue;
    }

    // Token (word)
    let word = '';
    while (i < cmd.length && !/[\s|;&><"']/.test(cmd[i])) {
      word += cmd[i];
      i++;
    }

    if (word) {
      // Classify token
      let type: TokenType = 'text';
      
      if (isFirst) {
        if (word === 'sudo') {
          type = 'sudo';
          sawSudo = true;
        } else if (sawSudo && /^-/.test(word)) {
          type = 'option';
        } else {
          type = 'command';
          isFirst = false;
          sawSudo = false;
        }
      } else if (word === 'sudo') {
        type = 'sudo';
      } else if (/^-/.test(word)) {
        type = 'option';
      } else if (/^\//.test(word) || /\//.test(word)) {
        type = 'path';
      }

      tokens.push({ type, value: word });
    }
  }

  return tokens;
}
