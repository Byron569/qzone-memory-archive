/**
 * Data contracts for bilateral recovery synchronisation.
 *
 * This module deliberately contains types and pure constants only.  It does
 * not make network requests, persist credentials, or implement cryptography.
 * A local provider and a future remote provider should implement the same
 * contract so that the UI can switch transport without changing merge logic.
 */

export const RECOVERY_SYNC_PROTOCOL_VERSION = 1 as const;
export const DEFAULT_SYNC_PAGE_SIZE = 100;
export const MAX_SYNC_PAGE_SIZE = 500;

export type RecoverySyncProviderKind = "local" | "remote";
export type RecoveryPairingRole = "initiator" | "invitee";
export type RecoveryPairingStatus = "pending" | "paired" | "revoked" | "expired";
export type RecoverySyncOperation = "upsert" | "tombstone";
export type RecoveryConflictStrategy = "prefer-local" | "prefer-remote" | "keep-both" | "manual";

/**
 * Opaque installation identity.  It is not the QQ number and must not be
 * derived from a cookie, g_tk, or any other login credential.
 */
export interface RecoveryDeviceIdentity {
  deviceId: string;
  displayName?: string;
  createdAt: number;
}

/**
 * A one-time pairing invitation.  The plaintext code should only be shown
 * locally (for example as a QR code); a server should store a slow hash.
 */
export interface RecoveryPairingCode {
  protocolVersion: typeof RECOVERY_SYNC_PROTOCOL_VERSION;
  pairingId: string;
  code: string;
  expiresAt: number;
  initiatorDeviceId: string;
  initiatorPublicKey: string;
}

export interface RecoveryPairingRequest {
  device: RecoveryDeviceIdentity;
  ttlSeconds?: number;
}

export interface RecoveryAcceptPairingRequest {
  pairingId: string;
  code: string;
  inviteeDevice: RecoveryDeviceIdentity;
  inviteePublicKey: string;
}

export interface RecoveryPairingSession {
  protocolVersion: typeof RECOVERY_SYNC_PROTOCOL_VERSION;
  pairingId: string;
  role: RecoveryPairingRole;
  status: RecoveryPairingStatus;
  localDeviceId: string;
  peerDeviceId?: string;
  peerPublicKey?: string;
  keyVersion: number;
  createdAt: number;
  expiresAt: number;
  revokedAt?: number;
}

/**
 * The client-side, decrypted representation of one observation.  This is
 * never sent to a remote server as plaintext.  `recordId` is a stable client
 * fingerprint (feed/cell/time based), not a QQ API session identifier.
 */
export interface RecoveryEvidenceRecord {
  recordId: string;
  sourceSide: "owner" | "counterpart";
  sourceAccountId: string;
  targetAccountId?: string;
  feedKey?: string;
  cellId?: string;
  eventType: number;
  eventTime?: number;
  title?: string;
  content?: string;
  eventSummary?: string;
  actorUin?: string;
  actorName?: string;
  originalAuthorUin?: string;
  originalAuthorName?: string;
  pictureFingerprints?: string[];
  videoFingerprint?: string;
  rawSourceDigest?: string;
  observedAt: number;
  revision: number;
}

/**
 * A server-visible envelope.  Only opaque identifiers, hashes, and encrypted
 * bytes may be present in this structure.  The payload is encrypted on the
 * client with the paired key before upload.
 */
export interface RecoveryEncryptedChange {
  protocolVersion: typeof RECOVERY_SYNC_PROTOCOL_VERSION;
  pairId: string;
  recordId: string;
  operation: RecoverySyncOperation;
  revision: number;
  keyVersion: number;
  ciphertextB64: string;
  nonceB64: string;
  aadB64?: string;
  payloadDigest: string;
  changedAt: number;
  deletedAt?: number;
}

export interface RecoverySyncCursor {
  streamId: string;
  position: string;
  issuedAt: number;
}

export interface RecoveryPushRequest {
  session: RecoveryPairingSession;
  changes: RecoveryEncryptedChange[];
  cursor?: RecoverySyncCursor;
}

export interface RecoveryPushResponse {
  accepted: number;
  rejected: number;
  nextCursor: RecoverySyncCursor;
  conflicts: RecoverySyncConflict[];
}

export interface RecoveryPullRequest {
  session: RecoveryPairingSession;
  cursor?: RecoverySyncCursor;
  limit?: number;
}

export interface RecoveryPullResponse {
  changes: RecoveryEncryptedChange[];
  nextCursor: RecoverySyncCursor;
  hasMore: boolean;
}

export interface RecoverySyncConflict {
  recordId: string;
  localRevision: number;
  remoteRevision: number;
  localDigest: string;
  remoteDigest: string;
  strategy: RecoveryConflictStrategy;
  requiresUserChoice: boolean;
}

export interface RecoverySyncAckRequest {
  session: RecoveryPairingSession;
  cursor: RecoverySyncCursor;
  recordIds?: string[];
}

/**
 * Provider contract shared by an offline file provider and a future HTTPS
 * provider.  Implementations should keep transport concerns here and leave
 * decrypting, validating, matching, and merging to the recovery service.
 */
export interface RecoverySyncProvider {
  readonly kind: RecoverySyncProviderKind;

  createPairingCode(request: RecoveryPairingRequest): Promise<RecoveryPairingCode>;
  acceptPairing(request: RecoveryAcceptPairingRequest): Promise<RecoveryPairingSession>;
  getPairing(pairingId: string): Promise<RecoveryPairingSession | null>;
  revokePairing(pairingId: string): Promise<void>;

  pushChanges(request: RecoveryPushRequest): Promise<RecoveryPushResponse>;
  pullChanges(request: RecoveryPullRequest): Promise<RecoveryPullResponse>;
  acknowledgeChanges(request: RecoverySyncAckRequest): Promise<void>;
}

/**
 * Additional operations useful to the local JSON/file provider.  The local
 * provider can implement these without a network or a public IP address.
 */
export interface LocalRecoverySyncProvider extends RecoverySyncProvider {
  readonly kind: "local";
  exportBundle(session: RecoveryPairingSession): Promise<RecoverySyncBundle>;
  importBundle(bundle: RecoverySyncBundle): Promise<RecoverySyncImportResult>;
}

/**
 * A portable local bundle.  If it contains evidence records, it is expected
 * to be encrypted before leaving the device; plaintext export is only for a
 * user-selected local backup path.
 */
export interface RecoverySyncBundle {
  protocolVersion: typeof RECOVERY_SYNC_PROTOCOL_VERSION;
  bundleId: string;
  pairingId?: string;
  sourceDeviceId: string;
  createdAt: number;
  cursor?: RecoverySyncCursor;
  encryptedChanges: RecoveryEncryptedChange[];
}

export interface RecoverySyncImportResult {
  bundleId: string;
  imported: number;
  skipped: number;
  conflicts: RecoverySyncConflict[];
  nextCursor?: RecoverySyncCursor;
}

export interface RemoteRecoverySyncProvider extends RecoverySyncProvider {
  readonly kind: "remote";
  readonly endpoint: string;
}

/**
 * Keep page sizes bounded even when a remote provider is implemented later.
 * This is intentionally pure and does not validate or contact a server.
 */
export function normalizeSyncPageSize(limit?: number): number {
  if (!Number.isFinite(limit)) return DEFAULT_SYNC_PAGE_SIZE;
  return Math.min(MAX_SYNC_PAGE_SIZE, Math.max(1, Math.floor(limit as number)));
}
