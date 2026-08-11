import type { Rule, Forward } from "./types";
import { emptyRule } from "./types";

export interface ParseResult {
  rule: Rule;
  errors: string[];
}

export function parseSshCommand(command: string): ParseResult {
  const errors: string[] = [];
  const rule = emptyRule();
  rule.forwards = [];

  const trimmed = command.trim();
  if (!trimmed) {
    return { rule, errors: ["Empty command"] };
  }

  const tokens = tokenize(trimmed);
  console.log("Tokens:", tokens);
  console.log(trimmed);

  if (tokens.length === 0 || tokens[0] !== "ssh") {
    errors.push("Command must start with 'ssh'");
    return { rule, errors };
  }

  let i = 1;
  while (i < tokens.length) {
    const token = tokens[i];

    if (token === "-p" && i + 1 < tokens.length) {
      const port = parseInt(tokens[i + 1], 10);
      if (isNaN(port) || port < 1 || port > 65535) {
        errors.push(`Invalid SSH port: ${tokens[i + 1]}`);
      } else {
        rule.sshPort = port;
      }
      i += 2;
    } else if (token === "-i" && i + 1 < tokens.length) {
      rule.sshKeyPath = tokens[i + 1];
      i += 2;
    } else if (token === "-L" && i + 1 < tokens.length) {
      const fwd = parseForwardSpec("local", tokens[i + 1], errors);
      if (fwd) rule.forwards.push(fwd);
      i += 2;
    } else if (token === "-R" && i + 1 < tokens.length) {
      const fwd = parseForwardSpec("remote", tokens[i + 1], errors);
      if (fwd) rule.forwards.push(fwd);
      i += 2;
    } else if (token === "-D" && i + 1 < tokens.length) {
      const fwd = parseDynamicSpec(tokens[i + 1], errors);
      if (fwd) rule.forwards.push(fwd);
      i += 2;
    } else if (
      token === "-N" ||
      token === "-f" ||
      token === "-q" ||
      token === "-v" ||
      token === "-T" ||
      token === "-t"
    ) {
      i += 1;
    } else if (token === "-o" && i + 1 < tokens.length) {
      i += 2;
    } else if (token.startsWith("-") && token.length === 2 && i + 1 < tokens.length) {
      i += 2;
    } else if (token.startsWith("-")) {
      i += 1;
    } else {
      const userHost = parseUserHost(token);
      if (userHost) {
        rule.sshUser = userHost.user;
        rule.sshHost = userHost.host;
      } else {
        errors.push(`Unexpected argument: ${token}`);
      }
      i += 1;
    }
  }

  if (!rule.sshHost) {
    errors.push("No host found (expected user@host or hostname)");
  }

  if (rule.forwards.length === 0) {
    errors.push("No port forwards found (-L, -R, or -D)");
  }

  return { rule, errors };
}

function tokenize(command: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let inSingle = false;
  let inDouble = false;

  for (let i = 0; i < command.length; i++) {
    const ch = command[i];

    if (ch === "'" && !inDouble) {
      inSingle = !inSingle;
    } else if (ch === '"' && !inSingle) {
      inDouble = !inDouble;
    } else if ((ch === " " || ch === "\t" || ch === "\n" || ch === "\r") && !inSingle && !inDouble) {
      if (current) {
        tokens.push(current);
        current = "";
      }
    } else if (ch === "\\" && i + 1 < command.length && !inSingle) {
      current += command[++i];
    } else {
      current += ch;
    }
  }

  if (current) tokens.push(current);
  return tokens;
}

function parseForwardSpec(
  type: "local" | "remote",
  spec: string,
  errors: string[],
): Forward | null {
  const parts = spec.split(":");

  if (parts.length === 3) {
    const listenPort = parseInt(parts[0], 10);
    const destHost = parts[1];
    const destPort = parseInt(parts[2], 10);

    if (isNaN(listenPort) || isNaN(destPort)) {
      errors.push(`Invalid forward spec: ${spec}`);
      return null;
    }

    return {
      forwardType: type,
      localPort: type === "local" ? listenPort : undefined,
      remotePort: type === "remote" ? listenPort : undefined,
      destinationHost: destHost,
      destinationPort: destPort,
      bindAddress: "127.0.0.1",
    };
  }

  if (parts.length === 4) {
    const bindAddr = parts[0];
    const listenPort = parseInt(parts[1], 10);
    const destHost = parts[2];
    const destPort = parseInt(parts[3], 10);

    if (isNaN(listenPort) || isNaN(destPort)) {
      errors.push(`Invalid forward spec: ${spec}`);
      return null;
    }

    return {
      forwardType: type,
      localPort: type === "local" ? listenPort : undefined,
      remotePort: type === "remote" ? listenPort : undefined,
      destinationHost: destHost,
      destinationPort: destPort,
      bindAddress: bindAddr,
    };
  }

  errors.push(`Invalid forward spec: ${spec} (expected [bind:]port:host:port)`);
  return null;
}

function parseDynamicSpec(spec: string, errors: string[]): Forward | null {
  const parts = spec.split(":");

  let port: number;
  let bindAddr = "127.0.0.1";

  if (parts.length === 1) {
    port = parseInt(parts[0], 10);
  } else if (parts.length === 2) {
    bindAddr = parts[0];
    port = parseInt(parts[1], 10);
  } else {
    errors.push(`Invalid dynamic forward spec: ${spec}`);
    return null;
  }

  if (isNaN(port)) {
    errors.push(`Invalid dynamic forward port: ${spec}`);
    return null;
  }

  return {
    forwardType: "dynamic",
    localPort: port,
    remotePort: undefined,
    destinationHost: undefined,
    destinationPort: undefined,
    bindAddress: bindAddr,
  };
}

function parseUserHost(
  token: string,
): { user: string; host: string } | null {
  if (token.includes("@")) {
    const [user, host] = token.split("@", 2);
    if (user && host) return { user, host };
  }
  if (!token.startsWith("-") && (token.includes(".") || token.includes(":"))) {
    return { user: "", host: token };
  }
  return null;
}
