export type ForwardType = "local" | "remote" | "dynamic";

export interface Forward {
  forwardType: ForwardType;
  localPort?: number;
  remotePort?: number;
  destinationHost?: string;
  destinationPort?: number;
  bindAddress?: string;
}

export interface Rule {
  id: string;
  name: string;
  sshHost: string;
  sshPort: number;
  sshUser: string;
  sshKeyPath: string;
  forwards: Forward[];
}

export type TunnelStatus =
  | { status: "disconnected" }
  | { status: "connecting" }
  | { status: "connected" }
  | { status: "error"; message: string };

export interface RuleWithStatus extends Rule {
  tunnelStatus: TunnelStatus;
}

export interface StatusEvent {
  ruleId: string;
  status: TunnelStatus;
}

export function emptyForward(): Forward {
  return {
    forwardType: "local",
    localPort: undefined,
    remotePort: undefined,
    destinationHost: "",
    destinationPort: undefined,
    bindAddress: "127.0.0.1",
  };
}

export function emptyRule(): Rule {
  return {
    id: "",
    name: "",
    sshHost: "",
    sshPort: 22,
    sshUser: "",
    sshKeyPath: "",
    forwards: [emptyForward()],
  };
}

export function toCleanRule(source: Rule | RuleWithStatus): Rule {
  return {
    id: source.id,
    name: source.name,
    sshHost: source.sshHost,
    sshPort: source.sshPort,
    sshUser: source.sshUser,
    sshKeyPath: source.sshKeyPath,
    forwards: source.forwards.map((f) => ({
      forwardType: f.forwardType,
      localPort: f.localPort,
      remotePort: f.remotePort,
      destinationHost: f.destinationHost,
      destinationPort: f.destinationPort,
      bindAddress: f.bindAddress,
    })),
  };
}

export function forwardSummary(fwd: Forward): string {
  switch (fwd.forwardType) {
    case "local":
      return `L :${fwd.localPort ?? "?"} → ${fwd.destinationHost ?? "?"}:${fwd.destinationPort ?? "?"}`;
    case "remote":
      return `R :${fwd.remotePort ?? "?"} → ${fwd.destinationHost ?? "?"}:${fwd.destinationPort ?? "?"}`;
    case "dynamic":
      return `D :${fwd.localPort ?? "?"} (SOCKS)`;
  }
}

export function forwardTypeLabel(type: ForwardType): string {
  switch (type) {
    case "local":
      return "L";
    case "remote":
      return "R";
    case "dynamic":
      return "D";
  }
}
