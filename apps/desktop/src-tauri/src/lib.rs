use sonora_common::{init_logging, LogConfig, TrackId};
use sonora_core::{
    ArtworkCandidate, ArtworkQuery, CachedArtworkAsset, LyricsCandidate, LyricsCandidateQuery,
    PlaybackStatus, QueueItem, RankedCandidateMatch, SonoraApp, SonoraCommand, SonoraConfig,
};
use sonora_library::{AlbumDto, ArtistDto, LibrarySummary, ScanStats, SearchResult};
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    app: Mutex<Option<Arc<SonoraApp>>>,
}

fn get_app(state: &State<AppState>) -> Result<Arc<SonoraApp>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Sonora App is not initialized".to_string())
}

#[tauri::command]
fn get_system_status() -> String {
    "Sonora Core v0.1.0 Ready".to_string()
}

#[tauri::command]
fn scan_directory(state: State<AppState>, path: String) -> Result<ScanStats, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.scan_directory(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_library(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.search(&query, limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_library_summary(state: State<AppState>) -> Result<LibrarySummary, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_library_summary().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_tracks(
    state: State<AppState>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_all_tracks(limit.unwrap_or(500))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_albums(state: State<AppState>) -> Result<Vec<AlbumDto>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_all_albums().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_artists(state: State<AppState>) -> Result<Vec<ArtistDto>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_all_artists().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_album_tracks(state: State<AppState>, album_id: i64) -> Result<Vec<SearchResult>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_album_tracks(album_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_artist_tracks(state: State<AppState>, artist_id: i64) -> Result<Vec<SearchResult>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_artist_tracks(artist_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_track_artwork(
    state: State<AppState>,
    track_id: i64,
    thumbnail: Option<bool>,
) -> Result<Option<String>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_track_artwork(TrackId(track_id), thumbnail.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_album_artwork(
    state: State<AppState>,
    album_id: i64,
    thumbnail: Option<bool>,
) -> Result<Option<String>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.get_album_artwork(album_id, thumbnail.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_directory() -> Result<Option<String>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Select Music Directory")
        .pick_folder()
        .await;
    Ok(folder.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
async fn pick_audio_file() -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Select Audio File")
        .add_filter(
            "Audio Files",
            &["flac", "mp3", "wav", "ogg", "opus", "m4a", "aac", "alac"],
        )
        .pick_file()
        .await;
    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
fn play_track(state: State<AppState>, track_id: i64) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.play_track_id(TrackId(track_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn play_file(state: State<AppState>, path: String) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.play_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn play_album(state: State<AppState>, album_id: i64) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.play_album(album_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn pause_playback(state: State<AppState>) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.pause().map_err(|e| e.to_string())
}

#[tauri::command]
fn resume_playback(state: State<AppState>) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.resume().map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_playback(state: State<AppState>) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.stop().map_err(|e| e.to_string())
}

#[tauri::command]
fn seek_playback(state: State<AppState>, position_ms: u64) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.seek(position_ms).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_volume(state: State<AppState>, volume: f32) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.set_volume(volume).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_playback_status(state: State<AppState>) -> Result<PlaybackStatus, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    // Poll queue to auto-advance if track finished
    let _ = app.poll_queue();
    Ok(app.status())
}

#[tauri::command]
fn get_queue(state: State<AppState>) -> Result<Vec<QueueItem>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    Ok(app.get_queue())
}

#[tauri::command]
fn enqueue_track(state: State<AppState>, track_id: i64) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.enqueue_track(TrackId(track_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_from_queue(state: State<AppState>, index: usize) -> Result<Option<QueueItem>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.remove_from_queue(index).map_err(|e| e.to_string())
}

#[tauri::command]
fn move_queue_item(state: State<AppState>, from: usize, to: usize) -> Result<bool, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.move_queue_item(from, to).map_err(|e| e.to_string())
}

#[tauri::command]
fn play_queue_index(state: State<AppState>, index: usize) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.play_queue_index(index).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_queue(state: State<AppState>) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.clear_queue();
    Ok(())
}

#[tauri::command]
fn queue_next(state: State<AppState>) -> Result<bool, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.queue_next().map_err(|e| e.to_string())
}

#[tauri::command]
fn queue_previous(state: State<AppState>) -> Result<bool, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.queue_previous().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_visualizer_data(state: State<AppState>) -> Result<Vec<f32>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    Ok(app.visualizer_data())
}

#[tauri::command]
fn get_lyrics(
    state: State<AppState>,
    track_id: Option<i64>,
    file_path: Option<String>,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<u64>,
) -> Result<Option<sonora_lyrics::model::LyricsDocument>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;

    let query = sonora_lyrics::provider::TrackLyricsQuery {
        title,
        artist,
        album,
        duration_ms,
        file_path: file_path.map(std::path::PathBuf::from),
        track_id,
    };

    app.get_lyrics(query).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_lyrics_offset(
    state: State<AppState>,
    track_id: Option<i64>,
    file_path: Option<String>,
    offset_ms: i64,
) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    app.save_lyrics_offset(track_id, file_path.as_deref(), offset_ms)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_lrc_file() -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Select LRC Lyrics File")
        .add_filter("LRC Files", &["lrc", "txt"])
        .pick_file()
        .await;
    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
fn load_lrc_file(
    state: State<AppState>,
    path: String,
    track_id: Option<i64>,
    file_path: Option<String>,
    title: Option<String>,
    artist: Option<String>,
) -> Result<sonora_lyrics::model::LyricsDocument, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;
    use sonora_lyrics::parser::LyricsParser;
    let parser = sonora_lyrics::parser::LrcLyricsParser;
    let mut doc = parser.parse(&content).map_err(|e| e.to_string())?;
    if doc.title.is_none() {
        doc.title = title.clone();
    }
    if doc.artist.is_none() {
        doc.artist = artist.clone();
    }

    let guard = state.app.lock().map_err(|e| e.to_string())?;
    if let Some(app) = guard.as_ref() {
        let repo = sonora_library::LibraryRepository::new(app.db());
        let _ = repo.save_cached_lyrics(
            track_id,
            file_path.as_deref(),
            title.as_deref().unwrap_or("Unknown"),
            artist.as_deref(),
            &doc,
            "manual_file",
        );
    }

    Ok(doc)
}

#[tauri::command]
fn list_plugins(state: State<AppState>) -> Result<Vec<sonora_plugin::PluginInfo>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    let app = guard
        .as_ref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())?;
    Ok(app.plugin_host().list())
}

fn app_of<'a>(
    guard: &'a std::sync::MutexGuard<'a, Option<Arc<SonoraApp>>>,
) -> Result<&'a SonoraApp, String> {
    guard
        .as_deref()
        .ok_or_else(|| "Sonora App is not initialized".to_string())
}

#[tauri::command]
async fn enrichment_find_metadata(
    state: State<'_, AppState>,
    track_id: i64,
) -> Result<Vec<RankedCandidateMatch>, String> {
    let app = get_app(&state)?;
    app.find_metadata_candidates(TrackId(track_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn enrichment_apply_metadata(
    state: State<'_, AppState>,
    track_id: i64,
    candidate: RankedCandidateMatch,
) -> Result<(), String> {
    let app = get_app(&state)?;
    app.apply_metadata(TrackId(track_id), &candidate)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn enrichment_find_artwork(
    state: State<'_, AppState>,
    query: ArtworkQuery,
) -> Result<Vec<ArtworkCandidate>, String> {
    let app = get_app(&state)?;
    app.find_artwork_candidates(&query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn enrichment_apply_artwork(
    state: State<'_, AppState>,
    target_type: String,
    target_id: i64,
    image_url: String,
) -> Result<CachedArtworkAsset, String> {
    let app = get_app(&state)?;
    app.apply_artwork(&target_type, target_id, &image_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn enrichment_find_lyrics(
    state: State<'_, AppState>,
    query: LyricsCandidateQuery,
) -> Result<Vec<LyricsCandidate>, String> {
    let app = get_app(&state)?;
    app.find_lyrics_candidates(&query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn enrichment_apply_lyrics(
    state: State<'_, AppState>,
    track_id: Option<i64>,
    file_path: Option<String>,
    candidate: LyricsCandidate,
) -> Result<(), String> {
    let app = get_app(&state)?;
    app.apply_lyrics_candidate(track_id, file_path.as_deref(), &candidate)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn enrichment_export_lrc(
    state: State<'_, AppState>,
    track_id: Option<i64>,
    file_path: Option<String>,
    lrc_content: String,
) -> Result<String, String> {
    let app = get_app(&state)?;
    app.export_lrc_sidecar(track_id, file_path.as_deref(), &lrc_content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_catalog(state: State<AppState>) -> Result<sonora_registry::Catalog, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?.market_catalog().map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_refresh(state: State<AppState>) -> Result<sonora_registry::Catalog, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?.market_refresh().map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_installed(
    state: State<AppState>,
) -> Result<Vec<sonora_registry::InstalledEntry>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_installed()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_updates(state: State<AppState>) -> Result<Vec<sonora_registry::UpdateInfo>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?.market_updates().map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_install(
    state: State<AppState>,
    id: String,
    version: Option<String>,
) -> Result<sonora_registry::InstallReport, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_install(&id, version.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_update(
    state: State<AppState>,
    id: String,
) -> Result<sonora_registry::InstallReport, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_update(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_rollback(
    state: State<AppState>,
    id: String,
    version: Option<String>,
) -> Result<sonora_registry::RollbackReport, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_rollback(&id, version.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_uninstall(state: State<AppState>, id: String) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_uninstall(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_themes(
    state: State<AppState>,
) -> Result<Vec<sonora_registry::InstalledTheme>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?.market_themes().map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_theme_definition(
    state: State<AppState>,
    id: String,
) -> Result<sonora_registry::ThemeDefinition, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_theme_definition(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_theme_css(state: State<AppState>, id: String) -> Result<Option<String>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_theme_css(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_active_theme(state: State<AppState>) -> Result<Option<String>, String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_active_theme()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn marketplace_set_active_theme(state: State<AppState>, id: Option<String>) -> Result<(), String> {
    let guard = state.app.lock().map_err(|e| e.to_string())?;
    app_of(&guard)?
        .market_set_active_theme(id.as_deref())
        .map_err(|e| e.to_string())
}

pub fn run() {
    init_logging(&LogConfig::default());

    let config = SonoraConfig::default();
    let app_instance = SonoraApp::new(config.clone())
        .or_else(|_| SonoraApp::in_memory(config))
        .ok()
        .map(Arc::new);

    // Boot-time plugin discovery: validate every plugin directory, then
    // load + start each one. Failures are isolated per plugin and logged;
    // a bad plugin never blocks startup or audio.
    if let Some(app) = app_instance.as_ref() {
        let plugins_dir = app.config().data_dir.join("plugins");
        for id in app.discover_plugins(&plugins_dir) {
            if let Err(e) = app
                .handle_command(SonoraCommand::PluginLoad { id: id.clone() })
                .and_then(|()| app.handle_command(SonoraCommand::PluginStart { id: id.clone() }))
            {
                tracing::warn!("plugin '{id}' failed to start and will stay inactive: {e}");
            }
        }
        app.forward_plugin_events();
    }

    tauri::Builder::default()
        .manage(AppState {
            app: Mutex::new(app_instance),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_status,
            scan_directory,
            search_library,
            get_library_summary,
            get_all_tracks,
            get_all_albums,
            get_all_artists,
            get_album_tracks,
            get_artist_tracks,
            get_track_artwork,
            get_album_artwork,
            pick_directory,
            pick_audio_file,
            play_track,
            play_file,
            play_album,
            pause_playback,
            resume_playback,
            stop_playback,
            seek_playback,
            set_volume,
            get_playback_status,
            get_queue,
            enqueue_track,
            remove_from_queue,
            move_queue_item,
            play_queue_index,
            clear_queue,
            queue_next,
            queue_previous,
            get_visualizer_data,
            get_lyrics,
            save_lyrics_offset,
            pick_lrc_file,
            load_lrc_file,
            list_plugins,
            marketplace_catalog,
            marketplace_refresh,
            marketplace_installed,
            marketplace_updates,
            marketplace_install,
            marketplace_update,
            marketplace_rollback,
            marketplace_uninstall,
            marketplace_themes,
            marketplace_theme_definition,
            marketplace_theme_css,
            marketplace_active_theme,
            marketplace_set_active_theme,
            enrichment_find_metadata,
            enrichment_apply_metadata,
            enrichment_find_artwork,
            enrichment_apply_artwork,
            enrichment_find_lyrics,
            enrichment_apply_lyrics,
            enrichment_export_lrc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running sonora desktop application");
}
