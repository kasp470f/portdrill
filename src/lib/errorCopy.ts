import type { ErrorKind, Rule, RuleWithStatus } from "./types";

export type ErrorAction =
  | "reviewKey"
  | "edit"
  | "retry"
  | "reconnect"
  | "chooseKey"
  | "defaultKey"
  | "stopOther";

export interface ErrorCopy {
  title: string;
  explain: string;
  meta?: string;
  /** The first action is the primary button. */
  actions: { id: ErrorAction; label: string }[];
  /** For "stopOther": the rule holding the port. */
  otherRuleId?: string;
}

function firstNumber(text: string, pattern: RegExp): number | undefined {
  const m = text.match(pattern);
  return m ? Number(m[1]) : undefined;
}

/** Plain-English title, explanation and fixes for a failed rule. */
export function describeError(
  kind: ErrorKind,
  message: string,
  rule: Rule,
  allRules: RuleWithStatus[],
): ErrorCopy {
  const host = rule.sshHost;
  const where = rule.sshPort === 22 ? host : `${host}:${rule.sshPort}`;

  switch (kind) {
    case "hostKeyChanged":
      return {
        title: "Host key changed",
        explain: `${where} is presenting a different key than the one you saved. That's expected after a server rebuild. If nothing changed, don't connect: someone may be intercepting the connection.`,
        actions: [
          { id: "reviewKey", label: "Review key…" }
        ],
      };

    case "authRejected":
      return {
        title: "The server rejected the sign-in",
        explain: `${host} didn't accept the key for user “${rule.sshUser}”. Check the user name, and that this key's public half is in ~/.ssh/authorized_keys on the server.`,
        meta: `Tried key: ${rule.sshKeyPath || "your default key"}`,
        actions: [
          { id: "chooseKey", label: "Choose key file…" }
        ],
      };

    case "unreachable":
      return {
        title: "Can't reach the server",
        explain: `${host} didn't answer on port ${rule.sshPort}. If it's only reachable through a VPN or office network, connect to that first. Otherwise check the port number.`,
        actions: [
          { id: "retry", label: "Try again" }
        ],
      };

    case "unknownHost":
      return {
        title: "Server name not found",
        explain: `“${host}” doesn't point to any address. Check the spelling. If it's an internal name, you may need to be on the VPN.`,
        actions: [
          { id: "edit", label: "Edit host" },
          { id: "retry", label: "Try again" },
        ],
      };

    case "localPortBusy": {
      const port =
        firstNumber(message, /\]:(\d+)/) ??
        firstNumber(message, /port:?\s*(\d+)/i) ??
        rule.forwards.find((f) => f.forwardType !== "remote")?.localPort;
      const other = allRules.find(
        (r) =>
          r.id !== rule.id &&
          (r.tunnelStatus.status === "connected" || r.tunnelStatus.status === "connecting") &&
          r.forwards.some((f) => f.forwardType !== "remote" && f.localPort === port),
      );
      return {
        title: port ? `Local port ${port} is already in use` : "A local port is already in use",
        explain: other
          ? `Another rule, “${other.name}”, is using localhost:${port}. Stop that rule or give this one a different local port.`
          : `Another program on this computer is using localhost:${port ?? "?"}. Close it, or give this rule a different local port.`,
        actions: other
          ? [
              { id: "stopOther", label: `Stop “${other.name}”` },
              { id: "edit", label: "Change local port" },
            ]
          : [{ id: "edit", label: "Change local port" }],
        otherRuleId: other?.id,
      };
    }

    case "remotePortRefused": {
      const port = firstNumber(message, /listen port (\d+)/i) ?? rule.forwards.find((f) => f.forwardType === "remote")?.remotePort;
      return {
        title: `The server wouldn't open port ${port ?? "?"}`,
        explain: `${host} refused the remote forward. Port ${port ?? "?"} may already be in use on the server, or the server may not allow remote forwarding (AllowTcpForwarding in sshd_config).`,
        actions: [{ id: "edit", label: "Change server port" }],
      };
    }

    case "keyFile":
      return {
        title: "Key file not found",
        explain: rule.sshKeyPath
          ? `This rule uses ${rule.sshKeyPath}, but that file doesn't exist or can't be read. It may have been moved or renamed.`
          : "Your default key couldn't be read.",
        actions: rule.sshKeyPath
          ? [
              { id: "chooseKey", label: "Choose key file…" },
              { id: "defaultKey", label: "Use my default key" },
            ]
          : [{ id: "chooseKey", label: "Choose key file…" }],
      };

    case "connectionLost":
      return {
        title: "Connection lost",
        explain:
          "This tunnel was running, then the connection dropped. That often happens after the computer sleeps or the network changes.",
        actions: [{ id: "reconnect", label: "Reconnect" }],
      };

    case "sshMissing":
      return {
        title: "OpenSSH isn't installed",
        explain: "PortDrill uses your computer's ssh program and couldn't find it.",
        actions: [{ id: "retry", label: "Try again" }],
      };

    default:
      return {
        title: "Couldn't connect",
        explain: "ssh stopped before the tunnel came up. Its last message is below.",
        actions: [
          { id: "retry", label: "Try again" }
        ],
      };
  }
}
