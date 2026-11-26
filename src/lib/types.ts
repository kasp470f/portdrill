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
  /** Display group; empty string means "no group". */
  group: string;
  /** Whether the rule card collapses its forwards (remembered per rule). */
  forwardsHidden: boolean;
  /** Bring the tunnel back when it drops after having been up. */
  autoReconnect: boolean;
  /** Connect when PortDrill starts. */
  connectOnLaunch: boolean;
}

export type ErrorKind =
  | "hostKeyChanged"
  | "authRejected"
  | "unreachable"
  | "unknownHost"
  | "localPortBusy"
  | "remotePortRefused"
  | "keyFile"
  | "connectionLost"
  | "sshMissing"
  | "other";

export type TunnelStatus =
  | { status: "disconnected" }
  | { status: "connecting" }
  | { status: "connected" }
  | {
      /** Dropped; auto-reconnect is waiting for, or making, attempt `attempt`. */
      status: "reconnecting";
      attempt: number;
      maxAttempts: number;
      /** Why the last connection or attempt ended. */
      message: string;
      kind: ErrorKind;
      /** Retrying the start-up connect, not a dropped tunnel. */
      onLaunch?: boolean;
    }
  | {
      status: "error";
      message: string;
      hostKeyChanged: boolean;
      kind: ErrorKind;
      /** Reconnect attempts that failed before this error; 0 without auto-reconnect. */
      reconnectAttempts?: number;
      /** The connect that failed was the one PortDrill makes when it starts. */
      onLaunch?: boolean;
    };

/** The ssh process may be running or about to be: on, starting, or coming back. */
export function isActiveStatus(status: TunnelStatus): boolean {
  return status.status === "connected" || status.status === "connecting" || status.status === "reconnecting";
}

/** What a host is presenting right now, shown before clearing the saved key. */
export interface HostKeyInfo {
  /** The known_hosts entry name, e.g. `[bastion.example.com]:2222`. */
  target: string;
  fingerprints: string[];
  scanError: string | null;
  /** What known_hosts currently has saved for this host. */
  savedFingerprints: string[];
}

export interface RuleWithStatus extends Rule {
  tunnelStatus: TunnelStatus;
  /** Paused from the row menu until PortDrill restarts. Never saved. */
  autoReconnectPaused: boolean;
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
    group: "",
    // New rules (filled in or pasted) start with only their badges showing.
    // Rules saved before this keep their own setting; a duplicate copies it.
    forwardsHidden: true,
    autoReconnect: false,
    connectOnLaunch: false,
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
    group: source.group ?? "",
    forwardsHidden: source.forwardsHidden ?? false,
    autoReconnect: source.autoReconnect ?? false,
    connectOnLaunch: source.connectOnLaunch ?? false,
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

/** The port this forward opens on the local side, if any (Local and SOCKS). */
export function localListenPort(fwd: Forward): number | undefined {
  return fwd.forwardType === "remote" ? undefined : fwd.localPort;
}

const typeOrder: Record<ForwardType, number> = { local: 0, remote: 1, dynamic: 2 };

/** Forwards sorted Local, Remote, SOCKS, as the card shows them. */
export function sortedForwards(forwards: Forward[]): Forward[] {
  return [...forwards].sort((a, b) => typeOrder[a.forwardType] - typeOrder[b.forwardType]);
}
