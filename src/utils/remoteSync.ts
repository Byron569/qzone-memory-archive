import { invoke } from "@tauri-apps/api/core";

export interface RemoteSyncConfig {
  endpoint?: string | null;
  deviceId?: string | null;
  serverDeviceId?: string | null;
  label?: string | null;
  accountUin?: string | null;
  keyVersion?: number | null;
  registered: boolean;
}

export interface RemoteDeviceRegistration {
  endpoint: string;
  deviceId: string;
  serverDeviceId: string;
  accountUin: string;
  keyVersion: number;
  createdAt: string;
}

export interface RemoteAccountPublicKey {
  keyVersion: number;
  publicKey: string;
  deviceId: string;
  label?: string | null;
  registeredAt: string;
}

export interface RemotePairingInvitation {
  pairingId: string;
  code: string;
  expiresAt: string;
  initiatorDeviceId: string;
  initiatorPublicKey: string;
}

export interface RemotePairing {
  pairingId: string;
  initiatorDeviceId: string;
  initiatorPublicKey?: string | null;
  claimantDeviceId?: string | null;
  claimantPublicKey?: string | null;
  status: string;
  expiresAt: string;
  createdAt: string;
  acceptedAt?: string | null;
}

export interface RemoteEncryptedChange {
  recordId: string;
  operation: "upsert" | "tombstone";
  revision: number;
  keyVersion: number;
  senderKeyVersion: number;
  targetUin: string;
  sourceUin?: string | null;
  ciphertextB64: string;
  nonceB64: string;
  aadB64?: string | null;
  payloadDigest: string;
  changedAt: number;
  deletedAt?: number | null;
}

export interface RemoteSyncCursor {
  streamId: string;
  position: string;
  issuedAt: number;
}

export interface RemoteSyncConflict {
  recordId: string;
  localRevision: number;
  remoteRevision: number;
  localDigest: string;
  remoteDigest: string;
}

export interface RemotePushResponse {
  accepted: number;
  rejected: number;
  nextCursor: RemoteSyncCursor;
  conflicts: RemoteSyncConflict[];
}

export interface RemotePullResponse {
  changes: RemoteEncryptedChange[];
  nextCursor: RemoteSyncCursor;
  hasMore: boolean;
}

export const SYNC_SERVER_ENDPOINT = "https://byron569.online";

export const getRemoteSyncConfig = () => invoke<RemoteSyncConfig>("get_remote_sync_config");
export const saveRemoteSyncEndpoint = (endpoint: string) =>
  invoke<RemoteSyncConfig>("save_remote_sync_endpoint", { endpoint });
export const registerRemoteDevice = (endpoint: string, accountUin: string, label?: string) =>
  invoke<RemoteDeviceRegistration>("register_remote_device", {
    endpoint,
    accountUin,
    label: label?.trim() || null,
  });
export const ensureRemoteDeviceRegistered = () =>
  invoke<RemoteSyncConfig>("ensure_remote_device_registered");
export const getAccountPublicKeys = (accountUin: string) =>
  invoke<RemoteAccountPublicKey[]>("get_account_public_keys", { accountUin });

// Legacy pairing commands are kept exported for older flows; the new UI no longer uses them.
export const createRemotePairing = () => invoke<RemotePairingInvitation>("create_remote_pairing");
export const claimRemotePairing = (code: string) => invoke<RemotePairing>("claim_remote_pairing", { code });
export const listRemotePairings = () => invoke<RemotePairing[]>("list_remote_pairings");

export const encryptRemotePayload = (request: {
  targetUin: string;
  peerPublicKey: string;
  targetKeyVersion: number;
  senderKeyVersion: number;
  recordId: string;
  operation: "upsert" | "tombstone";
  revision: number;
  payload: unknown;
  deletedAt?: number;
}) => invoke<RemoteEncryptedChange>("encrypt_remote_payload", { request });
export const decryptRemotePayload = (request: { sourceUin: string; change: RemoteEncryptedChange }) =>
  invoke<unknown>("decrypt_remote_payload", { request });
export const encryptRecoverySyncBatch = (request: {
  targetUin: string;
  peerPublicKey: string;
  targetKeyVersion: number;
  senderKeyVersion: number;
  package: unknown;
  observations: unknown[];
}) => invoke<RemoteEncryptedChange[]>("encrypt_recovery_sync_batch", { request });
export const pushRemoteChanges = (targetUin: string, changes: RemoteEncryptedChange[]) =>
  invoke<RemotePushResponse>("push_remote_changes", { targetUin, changes });
export const pullRemoteChanges = (cursor?: string, limit = 100) =>
  invoke<RemotePullResponse>("pull_remote_changes", { cursor: cursor || null, limit });
export const ackRemoteChanges = (cursor: string) =>
  invoke<void>("ack_remote_changes", { cursor });
export const clearRemoteSyncCredentials = () => invoke<void>("clear_remote_sync_credentials");
