import { invoke } from "@tauri-apps/api/core";

export interface RemoteSyncConfig {
  endpoint?: string | null;
  deviceId?: string | null;
  serverDeviceId?: string | null;
  label?: string | null;
  registered: boolean;
}

export interface RemoteDeviceRegistration {
  endpoint: string;
  deviceId: string;
  serverDeviceId: string;
  createdAt: string;
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

export const getRemoteSyncConfig = () => invoke<RemoteSyncConfig>("get_remote_sync_config");
export const saveRemoteSyncEndpoint = (endpoint: string) =>
  invoke<RemoteSyncConfig>("save_remote_sync_endpoint", { endpoint });
export const registerRemoteDevice = (endpoint: string, registrationToken: string, label?: string) =>
  invoke<RemoteDeviceRegistration>("register_remote_device", {
    endpoint,
    registrationToken,
    label: label?.trim() || null,
  });
export const createRemotePairing = () => invoke<RemotePairingInvitation>("create_remote_pairing");
export const claimRemotePairing = (code: string) => invoke<RemotePairing>("claim_remote_pairing", { code });
export const listRemotePairings = () => invoke<RemotePairing[]>("list_remote_pairings");
export const encryptRemotePayload = (request: {
  pairingId: string;
  peerPublicKey: string;
  recordId: string;
  operation: "upsert" | "tombstone";
  revision: number;
  keyVersion?: number;
  payload: unknown;
  deletedAt?: number;
}) => invoke<RemoteEncryptedChange>("encrypt_remote_payload", {
  request: { ...request, keyVersion: request.keyVersion ?? 1 },
});
export const decryptRemotePayload = (pairingId: string, peerPublicKey: string, change: RemoteEncryptedChange) =>
  invoke<unknown>("decrypt_remote_payload", {
    request: { pairingId, peerPublicKey, change },
  });
export const encryptRecoverySyncBatch = (request: {
  pairingId: string;
  peerPublicKey: string;
  package: unknown;
  observations: unknown[];
}) => invoke<RemoteEncryptedChange[]>("encrypt_recovery_sync_batch", { request });
export const pushRemoteChanges = (pairingId: string, changes: RemoteEncryptedChange[]) =>
  invoke<RemotePushResponse>("push_remote_changes", { pairingId, changes });
export const pullRemoteChanges = (pairingId: string, cursor?: string, limit = 100) =>
  invoke<RemotePullResponse>("pull_remote_changes", { pairingId, cursor: cursor || null, limit });
export const ackRemoteChanges = (pairingId: string, cursor: string) =>
  invoke<void>("ack_remote_changes", { pairingId, cursor });
export const clearRemoteSyncCredentials = () => invoke<void>("clear_remote_sync_credentials");
