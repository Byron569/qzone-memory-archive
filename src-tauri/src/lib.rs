mod archive;
mod qlogin;
mod qzone;
mod remote_sync;

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(archive::ArchiveState::new())
        .manage(qlogin::QLoginState::new())
        .manage(qzone::RecycleAuthState::default())
        .manage(qzone::QzonePageTokenState::default())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            exit_app,
            qlogin::start_qr_login,
            qlogin::poll_qr_login,
            qlogin::get_login_status,
            qlogin::logout_qzone,
            qlogin::open_web_login,
            qlogin::check_web_login,
            qlogin::sync_cookies_to_webview,
            remote_sync::get_remote_sync_config,
            remote_sync::save_remote_sync_endpoint,
            remote_sync::register_remote_device,
            remote_sync::ensure_remote_device_registered,
            remote_sync::get_account_public_keys,
            remote_sync::create_remote_pairing,
            remote_sync::claim_remote_pairing,
            remote_sync::list_remote_pairings,
            remote_sync::encrypt_remote_payload,
            remote_sync::encrypt_recovery_sync_batch,
            remote_sync::decrypt_remote_payload,
            remote_sync::push_remote_changes,
            remote_sync::pull_remote_changes,
            remote_sync::ack_remote_changes,
            remote_sync::clear_remote_sync_credentials,
            qzone::fetch_first_feeds,
            qzone::fetch_more_feeds,
            qzone::open_recycle_password_window,
            qzone::prepare_recycle_password_window,
            qzone::check_recycle_password,
            qzone::close_recycle_password_window,
            qzone::list_recycle_albums,
            qzone::list_recycle_photos,
            qzone::load_recycle_photo_preview,
            qzone::list_qzone_albums,
            qzone::create_qzone_album,
            qzone::recover_recycle_album,
            qzone::recover_recycle_photos,
            archive::start_feed_archive,
            archive::get_archive_progress,
            archive::cancel_feed_archive,
            archive::list_archive_skips,
            archive::retry_archive_skip,
            archive::list_archived_feeds,
            archive::list_archive_years,
            archive::list_archived_media,
            archive::get_archived_feed,
            archive::count_archived_feeds,
            archive::export_archived_html,
            archive::load_archived_image,
            archive::load_archived_video,
            archive::get_archive_overview,
            archive::list_interactors,
            archive::list_contact_comment_threads,
            archive::sync_qzone_library,
            archive::list_qzone_library,
            archive::list_qzone_library_years,
            archive::get_interaction_ranking,
            archive::export_recovery_evidence,
            archive::import_recovery_evidence,
            archive::prepare_recovery_sync_package,
            archive::import_recovery_sync_package,
            archive::list_recovery_evidence_packages,
            archive::list_recovery_evidence_candidates,
            archive::merge_recovery_evidence_item,
            archive::list_remote_sync_targets,
            archive::prepare_auto_sync_chunk,
            archive::mark_remote_evidence_uploaded,
            archive::save_remote_pull_cursor,
            archive::get_remote_sync_state,
            archive::delete_archived_feeds,
            archive::clear_archived_feeds,
            archive::delete_all_app_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
