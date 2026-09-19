import { invoke } from "@tauri-apps/api/core";

export interface FeedPage {
  feeds: Record<string, unknown>[];
  attachInfo?: string;
  hasMore: boolean;
}

export function fetchFirstFeeds() {
  return invoke<FeedPage>("fetch_first_feeds");
}

export function fetchMoreFeeds(attachInfo: string) {
  return invoke<FeedPage>("fetch_more_feeds", { attachInfo });
}

export type ArchiveStatus = "idle" | "running" | "completed" | "cancelled" | "limited" | "error";
export interface ArchiveProgress { status: ArchiveStatus; pages: number; fetched: number; saved: number; skipped: number; message: string; retryAt?: number; }
export interface ArchiveSkipItem {
  id: number; pageNumber: number; cursorOffset: number; offsetAdvance: number; baseTime: number;
  error: string; skippedAt: number; retryCount: number; lastRetryAt?: number; resolvedAt?: number; recoveredRecords: number;
}
export interface ArchiveSkipRetryResult { success: boolean; message: string; recoveredRecords: number; }
export interface ArchiveItem {
  id: number; cellId: string; publishedAt: number; content?: string; authorUin?: string;
  authorName?: string; pictureUrls: string[]; videoUrl?: string; videoUrls: string[]; videoCoverUrl?: string; likeCount: number; commentCount: number;
  likes: LikeUser[];
  comments: ArchiveComment[];
}
export interface LikeUser { uin?: string; nickname?: string; historical: boolean; likedAt: number; }
export interface ArchiveReply { uin?: string; nickname?: string; replyToUin?: string; replyToNickname?: string; content: string; createdAt: number; }
export interface ArchiveComment { uin?: string; nickname?: string; content: string; createdAt: number; replies: ArchiveReply[]; }
export type ArchiveCategory = "self" | "other" | "guestbook";
export interface ArchiveMediaItem { key: string; dynamicId: number; mediaType: "photo" | "video"; pictureIndex?: number; url: string; coverUrl?: string; publishedAt: number; authorUin?: string; authorName?: string; content?: string; }
export interface ArchiveMediaPage { items: ArchiveMediaItem[]; total: number; years: number[]; }
export const startFeedArchive = (intervalMs: number) => invoke<ArchiveProgress>("start_feed_archive", { intervalMs });
export const getArchiveProgress = () => invoke<ArchiveProgress>("get_archive_progress");
export const cancelFeedArchive = () => invoke<void>("cancel_feed_archive");
export const listArchiveSkips = () => invoke<ArchiveSkipItem[]>("list_archive_skips");
export const retryArchiveSkip = (id: number) => invoke<ArchiveSkipRetryResult>("retry_archive_skip", { id });
export const listArchivedFeeds = (limit = 100, offset = 0, category: ArchiveCategory = "self", year?: number, descending = true, query?: string) => invoke<ArchiveItem[]>("list_archived_feeds", { limit, offset, category, year, descending, query });
export const listArchiveYears = (category: ArchiveCategory = "self") => invoke<number[]>("list_archive_years", { category });
export const listArchivedMedia = (limit = 60, offset = 0, year?: number) => invoke<ArchiveMediaPage>("list_archived_media", { limit, offset, year });
export const getArchivedFeed = (id: number) => invoke<ArchiveItem>("get_archived_feed", { id });
export const countArchivedFeeds = (category: ArchiveCategory = "self", year?: number, query?: string) => invoke<number>("count_archived_feeds", { category, year, query });
export const exportArchivedHtml = (category: ArchiveCategory, ids?: number[]) => invoke<string>("export_archived_html", { category, ids });
export const loadArchivedImage = (id: number, pictureIndex: number) => invoke<string>("load_archived_image", { id, pictureIndex });
export const loadArchivedVideo = (id: number) => invoke<string>("load_archived_video", { id });
export interface ArchiveOverview { dynamics: number; pictures: number; comments: number; likes: number; databaseBytes: number; }
export const getArchiveOverview = () => invoke<ArchiveOverview>("get_archive_overview");
export interface Interactor { uin: string; nickname: string; likes: number; comments: number; total: number; lastAt: number; }
export const listInteractors = () => invoke<Interactor[]>("list_interactors");
export const listContactCommentThreads = (uin: string) => invoke<ArchiveItem[]>("list_contact_comment_threads", { uin });
export interface InteractionRank { uin: string; nickname: string; interactions: number; likes: number; comments: number; }
export const getInteractionRanking = (limit = 8) => invoke<InteractionRank[]>("get_interaction_ranking", { limit });
export const deleteArchivedFeeds = (ids: number[]) => invoke<number>("delete_archived_feeds", { ids });
export const clearArchivedFeeds = () => invoke<number>("clear_archived_feeds");
export const deleteAllAppData = () => invoke<void>("delete_all_app_data");

/**
 * A summary of a local, credential-free evidence package exchanged between
 * two authorized accounts. The package itself is written/read by Rust after
 * the user has selected a path in the native file picker.
 */
export interface RecoveryEvidencePackageSummary {
  packageId: string;
  exporterUin: string;
  targetUin?: string | null;
  createdAt: number;
  importedAt?: number | null;
  itemCount: number;
}

export interface RecoveryEvidenceObservation {
  kind: "interaction" | "dynamic";
  sourceSide: string;
  sourceUin: string;
  targetUin?: string | null;
  eventKey: string;
  cellId?: string | null;
  eventType: number;
  eventTime: number;
  title?: string | null;
  content?: string | null;
  eventSummary?: string | null;
  actorUin?: string | null;
  actorName?: string | null;
  originalAuthorUin?: string | null;
  originalAuthorName?: string | null;
  pictureCount: number;
  picturesJson?: string | null;
  videoJson?: string | null;
  commentsJson?: string | null;
  category?: string | null;
  rawJson: string;
}

export interface RecoveryEvidenceSyncPackage {
  schemaVersion: number;
  packageId: string;
  exporterUin: string;
  targetUin?: string | null;
  createdAt: number;
  observations: RecoveryEvidenceObservation[];
}

export const exportRecoveryEvidence = (path: string, targetUin?: string) =>
  invoke<RecoveryEvidencePackageSummary>("export_recovery_evidence", { path, targetUin: targetUin || null });
export const importRecoveryEvidence = (path: string) =>
  invoke<RecoveryEvidencePackageSummary>("import_recovery_evidence", { path });
export const listRecoveryEvidencePackages = () =>
  invoke<RecoveryEvidencePackageSummary[]>("list_recovery_evidence_packages");
export const prepareRecoverySyncPackage = (targetUin: string) =>
  invoke<RecoveryEvidenceSyncPackage>("prepare_recovery_sync_package", { targetUin });
export const importRecoverySyncPackage = (pkg: RecoveryEvidenceSyncPackage) =>
  invoke<RecoveryEvidencePackageSummary>("import_recovery_sync_package", { package: pkg });

/**
 * A record found in an imported evidence package that is not yet present in
 * the current account's local archive. Candidates are deliberately exposed
 * as a review model rather than silently merging them into archive_feeds.
 */
export interface RecoveryEvidenceCandidate {
  id: number;
  packageId: string;
  sourceUin: string;
  sourceSide: string;
  eventKey: string;
  cellId?: string | null;
  eventType: number;
  eventTime: number;
  title?: string | null;
  content?: string | null;
  eventSummary?: string | null;
  actorUin?: string | null;
  actorName?: string | null;
  originalAuthorUin?: string | null;
  originalAuthorName?: string | null;
  pictureCount: number;
  category?: string | null;
  /** Backend matching confidence, e.g. "high", "medium", or "low". */
  confidence?: string | null;
  /** Human-readable explanation of why the record is a candidate. */
  matchReason?: string | null;
}

export interface RecoveryEvidenceCandidatePage {
  items: RecoveryEvidenceCandidate[];
  total: number;
}

export interface RecoveryEvidenceMergeResult {
  candidateId: number;
  merged: boolean;
  archiveId?: number | null;
  message: string;
}

export const listRecoveryEvidenceCandidates = async (_limit = 20, _offset = 0) => {
  const result = await invoke<
    RecoveryEvidenceCandidatePage
    | RecoveryEvidenceCandidate[]
    | { candidates: RecoveryEvidenceCandidate[]; total?: number }
  >("list_recovery_evidence_candidates", { limit: Math.min(100, Math.max(1, Math.floor(_limit))), offset: Math.max(0, Math.floor(_offset)) });
  if (Array.isArray(result)) return { items: result, total: result.length };
  if ("candidates" in result) return { items: result.candidates, total: result.total ?? result.candidates.length };
  return result;
};

export const mergeRecoveryEvidenceItem = (candidateId: number) =>
  invoke<RecoveryEvidenceMergeResult>("merge_recovery_evidence_item", { id: candidateId });

export const openRecyclePasswordWindow = () => invoke<void>("open_recycle_password_window");
export const prepareRecyclePasswordWindow = () => invoke<string>("prepare_recycle_password_window");
export const checkRecyclePassword = () => invoke<string | null>("check_recycle_password");
export const closeRecyclePasswordWindow = () => invoke<void>("close_recycle_password_window");
export const listRecycleAlbums = (pwd2sig: string) => invoke<Record<string, unknown>>("list_recycle_albums", { pwd2sig });
export const listRecyclePhotos = (pwd2sig: string, albumId?: string) => invoke<Record<string, unknown>>("list_recycle_photos", { pwd2sig, albumId });
export const listQzoneAlbums = () => invoke<Record<string, unknown>>("list_qzone_albums");
export const createQzoneAlbum = (name: string) => invoke<Record<string, unknown>>("create_qzone_album", { name });
export const recoverRecycleAlbum = (pwd2sig: string, albumId: string) => invoke<Record<string, unknown>>("recover_recycle_album", { pwd2sig, albumId });
export const recoverRecyclePhotos = (pwd2sig: string, sourceAlbumId: string, targetAlbumId: string, photoIds: string[]) =>
  invoke<Record<string, unknown>>("recover_recycle_photos", { pwd2sig, sourceAlbumId, targetAlbumId, photoIds });
export const loadRecyclePhotoPreview = (imageUrl: string) => invoke<string>("load_recycle_photo_preview", { imageUrl });

export type LibraryModule = "albums" | "photos" | "videos" | "guestbook" | "favorites";
export interface LibraryItem {
  id: number; module: LibraryModule; itemKey: string; parentKey: string; createdAt: number;
  title: string; summary: string; authorUin?: string; authorName?: string; coverUrl?: string; mediaUrls: string[];
}
export interface LibraryPage {
  items: LibraryItem[]; total: number; remoteTotal: number; complete: boolean; syncedAt: number; lastError?: string;
}
export interface LibrarySyncResult {
  module: LibraryModule; fetched: number; saved: number; remoteTotal: number; complete: boolean; message: string;
}
export const syncQzoneLibrary = (module: LibraryModule, parentKey?: string) =>
  invoke<LibrarySyncResult>("sync_qzone_library", { module, parentKey });
export const listQzoneLibrary = (module: LibraryModule, parentKey?: string, query?: string, year?: number, limit = 60, offset = 0) =>
  invoke<LibraryPage>("list_qzone_library", { module, parentKey, query, year, limit, offset });
export const listQzoneLibraryYears = (module: LibraryModule, parentKey?: string) =>
  invoke<number[]>("list_qzone_library_years", { module, parentKey });

// ---- Account-level auto sync (remote evidence upload/pull state) ----

export interface AutoSyncChunk {
  package: RecoveryEvidenceSyncPackage;
  hasMore: boolean;
  nextEventKey?: string | null;
}

export interface RemoteSyncState {
  accountUin?: string | null;
  registered: boolean;
  lastSyncAt?: number | null;
  uploadedCount: number;
  pendingCount: number;
  pullCursor?: string | null;
}

export const listRemoteSyncTargets = () => invoke<string[]>("list_remote_sync_targets");
export const prepareAutoSyncChunk = (
  targetUin: string,
  peerPubkey: string,
  limit: number,
  beforeEventKey?: string,
) =>
  invoke<AutoSyncChunk>("prepare_auto_sync_chunk", {
    targetUin,
    peerPubkey,
    limit,
    beforeEventKey: beforeEventKey || null,
  });
export const markRemoteEvidenceUploaded = (
  targetUin: string,
  packageId: string,
  peerPubkey: string,
  eventKeys: string[],
) => invoke<number>("mark_remote_evidence_uploaded", { targetUin, packageId, peerPubkey, eventKeys });
export const saveRemotePullCursor = (cursor: string) => invoke<void>("save_remote_pull_cursor", { cursor });
export const getRemoteSyncState = () => invoke<RemoteSyncState>("get_remote_sync_state");
