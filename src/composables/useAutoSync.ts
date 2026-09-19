import { ackRemoteChanges, decryptRemotePayload, encryptRecoverySyncBatch, ensureRemoteDeviceRegistered, getAccountPublicKeys, getRemoteSyncConfig, pullRemoteChanges, pushRemoteChanges, type RemoteAccountPublicKey } from "../utils/remoteSync";
import { SYNC_SERVER_ENDPOINT, saveRemoteSyncEndpoint } from "../utils/remoteSync";
import { getRemoteSyncState, importRecoverySyncPackage, listRemoteSyncTargets, markRemoteEvidenceUploaded, prepareAutoSyncChunk, saveRemotePullCursor, type RecoveryEvidenceSyncPackage } from "../utils/qzone";

export interface AutoSyncSummary {
  uploaded: number;
  pulled: number;
  skippedTargets: number;
}

function pickPeerKey(keys: RemoteAccountPublicKey[]): RemoteAccountPublicKey | null {
  if (!keys.length) return null;
  return keys.reduce((best, key) => (key.keyVersion > best.keyVersion ? key : best));
}

function syncPackageMetadata(pkg: RecoveryEvidenceSyncPackage) {
  return {
    schemaVersion: pkg.schemaVersion,
    packageId: pkg.packageId,
    exporterUin: pkg.exporterUin,
    targetUin: pkg.targetUin ?? null,
    createdAt: pkg.createdAt,
  };
}

/**
 * 账号级自动同步核心逻辑（无 UI）：自动注册 → 上传本账号互动记录 → 拉取发往本账号的证据。
 * 启动时静默调用（App.vue）和设置页手动触发（SettingsView）共用此实现。
 */
export async function runAutoSync(): Promise<AutoSyncSummary> {
  const config = await getRemoteSyncConfig();
  if (config?.endpoint?.trim() !== SYNC_SERVER_ENDPOINT) {
    await saveRemoteSyncEndpoint(SYNC_SERVER_ENDPOINT).catch(() => undefined);
  }

  const registered = await ensureRemoteDeviceRegistered();
  const senderKeyVersion = registered.keyVersion ?? 1;
  const state = await getRemoteSyncState();
  const targets = await listRemoteSyncTargets();

  let uploaded = 0;
  let pulled = 0;
  let skippedTargets = 0;

  // 上传：对每个互动对象账号加密上传未同步的证据
  for (const target of targets) {
    const keys = await getAccountPublicKeys(target);
    const peerKey = pickPeerKey(keys);
    if (!peerKey) {
      skippedTargets += 1;
      continue;
    }
    let beforeEventKey: string | undefined;
    let guard = 0;
    do {
      const chunk = await prepareAutoSyncChunk(target, peerKey.publicKey, 100, beforeEventKey);
      if (!chunk.package.observations.length) break;
      const changes = await encryptRecoverySyncBatch({
        targetUin: target,
        peerPublicKey: peerKey.publicKey,
        targetKeyVersion: peerKey.keyVersion,
        senderKeyVersion,
        package: syncPackageMetadata(chunk.package),
        observations: chunk.package.observations,
      });
      const result = await pushRemoteChanges(target, changes);
      uploaded += result.accepted;
      await markRemoteEvidenceUploaded(
        target,
        chunk.package.packageId,
        peerKey.publicKey,
        chunk.package.observations.map((item) => item.eventKey),
      );
      beforeEventKey = chunk.nextEventKey ?? undefined;
      guard += 1;
      if (guard > 5000) break;
    } while (beforeEventKey);
  }

  // 拉取：解密发往本账号的证据，重组包后导入候选队列
  let cursor = state.pullCursor ?? undefined;
  let guard = 0;
  do {
    const page = await pullRemoteChanges(cursor, 100);
    const packages = new Map<string, RecoveryEvidenceSyncPackage>();
    for (const change of page.changes) {
      try {
        const decoded = (await decryptRemotePayload({ sourceUin: change.sourceUin || "", change })) as Partial<RecoveryEvidenceSyncPackage> & {
          observation?: RecoveryEvidenceSyncPackage["observations"][number];
        };
        if (!decoded.packageId || !decoded.exporterUin || !decoded.observation) continue;
        const current = packages.get(decoded.packageId);
        if (current) current.observations.push(decoded.observation);
        else
          packages.set(decoded.packageId, {
            schemaVersion: decoded.schemaVersion || 1,
            packageId: decoded.packageId,
            exporterUin: decoded.exporterUin,
            targetUin: decoded.targetUin,
            createdAt: decoded.createdAt || Math.floor(Date.now() / 1000),
            observations: [decoded.observation],
          });
        pulled += 1;
      } catch (reason) {
        console.warn("跳过无法解密的远程证据", reason);
      }
    }
    for (const pkg of packages.values()) {
      await importRecoverySyncPackage(pkg);
    }
    cursor = page.nextCursor.position;
    if (!page.hasMore) {
      await ackRemoteChanges(cursor);
      await saveRemotePullCursor(cursor);
      break;
    }
    guard += 1;
    if (guard > 100) break;
  } while (true);

  return { uploaded, pulled, skippedTargets };
}
