//! End-to-end marketplace tests: schema, client, install, update, rollback.
//!
//! All network is scripted through [`FakeNetworkTransport`]; fixture packages
//! are built in-test. No filesystem location outside the per-test temp dir is
//! touched.

use sonora_plugin::{FakeNetworkTransport, FakeOutcome, PluginHost};
use sonora_registry::{ExtensionKind, Marketplace, MarketplacePaths};
use std::io::{Cursor, Write as _};
use std::path::PathBuf;
use std::sync::Arc;

const BASE: &str = "https://registry.test/v1";
const PLUGIN_ID: &str = "org.example.test";
const THEME_ID: &str = "org.example.night";

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sonora-mkt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture_wasm() -> Vec<u8> {
    wat::parse_str(
        r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1))
      (func (export "plugin_init") (result i32) (i32.const 0)))"#,
    )
    .unwrap()
}

fn plugin_manifest(id: &str, version: &str, caps: &[&str], domains: &[&str]) -> String {
    let caps_json = caps
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(",");
    let domains_json = domains
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"id\":\"{id}\",\"name\":\"Test Plugin\",\"version\":\"{version}\",\"author\":\"Tester\",\"description\":\"Fixture plugin.\",\"api_version\":1,\"capabilities\":[{caps_json}],\"entry\":\"plugin.wasm\",\"allowed_domains\":[{domains_json}]}}"
    )
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }
    buf
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn version_entry(version: &str, url: &str, sha: &str, changelog: &str) -> serde_json::Value {
    serde_json::json!({
        "version": version,
        "download_url": url,
        "sha256": sha,
        "changelog": changelog,
    })
}

fn plugin_entry(
    id: &str,
    caps: &[&str],
    latest: &str,
    versions: Vec<serde_json::Value>,
) -> serde_json::Value {
    let caps_json: Vec<_> = caps.iter().collect();
    serde_json::json!({
        "id": id,
        "name": "Test Plugin",
        "description": "Fixture plugin.",
        "author": {"name": "Tester"},
        "category": "utility",
        "capabilities": caps_json,
        "api_version": 1,
        "min_sonora_version": "0.1.0",
        "latest_version": latest,
        "versions": versions,
    })
}

fn theme_entry(id: &str, latest: &str, versions: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": "Night Theme",
        "description": "Fixture theme.",
        "author": {"name": "Tester"},
        "category": "dark",
        "min_sonora_version": "0.1.0",
        "latest_version": latest,
        "versions": versions,
    })
}

fn theme_tokens_json() -> String {
    let keys = [
        "--bg-base",
        "--bg-surface",
        "--bg-surface-hover",
        "--bg-elevated",
        "--bg-card",
        "--bg-card-hover",
        "--bg-active",
        "--bg-glass",
        "--accent-primary",
        "--accent-primary-hover",
        "--accent-primary-active",
        "--accent-secondary",
        "--accent-secondary-hover",
        "--accent-glow",
        "--accent-subtle",
        "--text-primary",
        "--text-secondary",
        "--text-muted",
        "--text-disabled",
        "--text-inverse",
        "--border-subtle",
        "--border-medium",
        "--border-focus",
        "--border-active",
        "--state-success",
        "--state-warning",
        "--state-danger",
        "--state-info",
    ];
    let pairs: Vec<String> = keys
        .iter()
        .map(|k| format!("\"{k}\":\"#112233\""))
        .collect();
    format!("{{{}}}", pairs.join(","))
}

fn theme_manifest(id: &str, version: &str) -> String {
    format!(
        "{{\"id\":\"{id}\",\"name\":\"Night Theme\",\"version\":\"{version}\",\"author\":\"Tester\",\"mode\":\"dark\",\"tokens\":{}}}",
        theme_tokens_json()
    )
}

struct Fixture {
    dir: PathBuf,
    fake: Arc<FakeNetworkTransport>,
    host: Arc<PluginHost>,
    market: Marketplace,
}

fn setup(
    plugins: Vec<serde_json::Value>,
    themes: Vec<serde_json::Value>,
    zips: Vec<(&str, Vec<u8>)>,
) -> Fixture {
    let dir = test_dir(&format!("{}-{}", std::line!(), now_nanos()));
    let fake = Arc::new(FakeNetworkTransport::new());
    let plugins_doc = serde_json::json!({"schema": 1, "plugins": plugins}).to_string();
    fake.route(
        "plugins.json",
        FakeOutcome::Status(200, plugins_doc.into_bytes()),
    );
    let themes_doc = serde_json::json!({"schema": 1, "themes": themes}).to_string();
    fake.route(
        "themes.json",
        FakeOutcome::Status(200, themes_doc.into_bytes()),
    );
    for (key, bytes) in zips {
        fake.route(key, FakeOutcome::Status(200, bytes));
    }
    let host = Arc::new(PluginHost::new().unwrap());
    let market = Marketplace::new(
        &dir,
        Arc::clone(&host),
        Some(BASE.to_string()),
        Some(fake.clone()),
    )
    .unwrap();
    Fixture {
        dir,
        fake,
        host,
        market,
    }
}

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn plugin_v1() -> (serde_json::Value, Vec<u8>) {
    let manifest = plugin_manifest(PLUGIN_ID, "1.0.0", &["storage:cache"], &[]);
    let zip = zip_of(&[
        ("manifest.json", manifest.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
    ]);
    let entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "1.0.0",
        vec![version_entry(
            "1.0.0",
            "https://cdn.test/org.example.test-1.0.0.zip",
            &sha256_hex(&zip),
            "First release.",
        )],
    );
    (entry, zip)
}

#[test]
fn install_plugin_end_to_end() {
    let (entry, zip) = plugin_v1();
    let f = setup(
        vec![entry],
        vec![],
        vec![("org.example.test-1.0.0.zip", zip)],
    );
    let report = f.market.install(PLUGIN_ID, None).unwrap();
    assert_eq!(report.version, "1.0.0");
    assert!(report.fresh_install);
    assert!(report.started);

    let dir = f.dir.join("plugins").join(PLUGIN_ID);
    assert!(dir.join("manifest.json").is_file());
    assert!(dir.join("plugin.wasm").is_file());
    assert_eq!(
        f.host.state(PLUGIN_ID),
        Some(sonora_plugin::PluginState::Running)
    );

    let installed = f.market.installed().unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].version, "1.0.0");
    assert_eq!(installed[0].state, "running");

    let catalog = f.market.catalog().unwrap();
    assert!(!catalog.offline);
    assert_eq!(catalog.plugins.len(), 1);
    assert_eq!(
        catalog.plugins[0].installed_version.as_deref(),
        Some("1.0.0")
    );
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn checksum_mismatch_installs_nothing() {
    let (mut entry, _zip) = plugin_v1();
    // Advertise a wrong digest for the real bytes.
    entry["versions"][0]["sha256"] =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    let (_e2, zip) = plugin_v1();
    let f = setup(
        vec![entry],
        vec![],
        vec![("org.example.test-1.0.0.zip", zip)],
    );
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(err.contains("checksum mismatch"), "unexpected: {err}");
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn traversal_zip_rejected() {
    let (entry, _) = plugin_v1();
    let evil = zip_of(&[
        (
            "manifest.json",
            plugin_manifest(PLUGIN_ID, "1.0.0", &["storage:cache"], &[]).as_bytes(),
        ),
        ("../evil.wasm", b"evil"),
    ]);
    let f = setup(
        vec![entry],
        vec![],
        vec![("org.example.test-1.0.0.zip", evil)],
    );
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(
        err.contains("checksum mismatch") || err.contains("unsafe"),
        "unexpected: {err}"
    );
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn traversal_zip_rejected_with_valid_checksum() {
    // Same as above but the attacker also updated the digest: extraction
    // guards must still refuse.
    let manifest = plugin_manifest(PLUGIN_ID, "1.0.0", &["storage:cache"], &[]);
    let evil = zip_of(&[
        ("manifest.json", manifest.as_bytes()),
        ("../evil.wasm", b"evil"),
    ]);
    let sha = sha256_hex(&evil);
    let entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "1.0.0",
        vec![version_entry(
            "1.0.0",
            "https://cdn.test/evil.zip",
            &sha,
            "Evil.",
        )],
    );
    let f = setup(vec![entry], vec![], vec![("cdn.test/evil.zip", evil)]);
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(err.contains("unsafe"), "unexpected: {err}");
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn capability_escalation_rejected() {
    let manifest = plugin_manifest(
        PLUGIN_ID,
        "1.0.0",
        &["storage:cache", "network:fetch"],
        &["https://example.com"],
    );
    let zip = zip_of(&[
        ("manifest.json", manifest.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
    ]);
    let entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "1.0.0",
        vec![version_entry(
            "1.0.0",
            "https://cdn.test/esc.zip",
            &sha256_hex(&zip),
            "Esc.",
        )],
    );
    let f = setup(vec![entry], vec![], vec![("cdn.test/esc.zip", zip)]);
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(
        err.contains("beyond the registry listing"),
        "unexpected: {err}"
    );
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn identity_mismatch_rejected() {
    let manifest = plugin_manifest("org.example.other", "1.0.0", &["storage:cache"], &[]);
    let zip = zip_of(&[
        ("manifest.json", manifest.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
    ]);
    let entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "1.0.0",
        vec![version_entry(
            "1.0.0",
            "https://cdn.test/mid.zip",
            &sha256_hex(&zip),
            "Mid.",
        )],
    );
    let f = setup(vec![entry], vec![], vec![("cdn.test/mid.zip", zip)]);
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(err.contains("mismatch"), "unexpected: {err}");
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn update_then_rollback() {
    let manifest_v1 = plugin_manifest(PLUGIN_ID, "1.0.0", &["storage:cache"], &[]);
    let zip_v1 = zip_of(&[
        ("manifest.json", manifest_v1.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
    ]);
    let manifest_v2 = plugin_manifest(PLUGIN_ID, "1.1.0", &["storage:cache"], &[]);
    let zip_v2 = zip_of(&[
        ("manifest.json", manifest_v2.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
        ("v2.txt", b"v2 marker"),
    ]);
    let entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "1.1.0",
        vec![
            version_entry(
                "1.0.0",
                "https://cdn.test/p-1.0.0.zip",
                &sha256_hex(&zip_v1),
                "First.",
            ),
            version_entry(
                "1.1.0",
                "https://cdn.test/p-1.1.0.zip",
                &sha256_hex(&zip_v2),
                "Second release with improvements.",
            ),
        ],
    );
    let f = setup(
        vec![entry],
        vec![],
        vec![("p-1.0.0.zip", zip_v1), ("p-1.1.0.zip", zip_v2)],
    );

    // Install the old version explicitly, then update.
    let r1 = f.market.install(PLUGIN_ID, Some("1.0.0")).unwrap();
    assert_eq!(r1.version, "1.0.0");
    assert!(r1.fresh_install);

    let updates = f.market.updates().unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].available, "1.1.0");
    assert!(updates[0].compatible);
    assert!(updates[0].changelog.contains("Second release"));

    let r2 = f.market.update(PLUGIN_ID).unwrap();
    assert_eq!(r2.version, "1.1.0");
    assert!(!r2.fresh_install);
    assert_eq!(r2.backed_up_version.as_deref(), Some("1.0.0"));
    let dir = f.dir.join("plugins").join(PLUGIN_ID);
    assert!(dir.join("v2.txt").is_file());
    assert!(f
        .dir
        .join("marketplace/backups")
        .join(PLUGIN_ID)
        .join("1.0.0")
        .is_dir());

    // Roll back to the previous known-good version.
    let rb = f.market.rollback(PLUGIN_ID, None).unwrap();
    assert_eq!(rb.restored_version, "1.0.0");
    assert!(!dir.join("v2.txt").exists());
    let installed = f.market.installed().unwrap();
    assert_eq!(installed[0].version, "1.0.0");
    assert_eq!(
        f.host.state(PLUGIN_ID),
        Some(sonora_plugin::PluginState::Running)
    );
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn uninstall_removes_files_and_receipt() {
    let (entry, zip) = plugin_v1();
    let f = setup(
        vec![entry],
        vec![],
        vec![("org.example.test-1.0.0.zip", zip)],
    );
    f.market.install(PLUGIN_ID, None).unwrap();
    f.market.uninstall(PLUGIN_ID).unwrap();
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    assert!(f.market.installed().unwrap().is_empty());
    assert_eq!(
        f.host.state(PLUGIN_ID),
        Some(sonora_plugin::PluginState::Unloaded)
    );
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn theme_install_has_no_code_path() {
    let manifest = theme_manifest(THEME_ID, "2.0.0");
    let zip = zip_of(&[
        ("theme.json", manifest.as_bytes()),
        ("theme.css", b":root{}"),
    ]);
    let entry = theme_entry(
        THEME_ID,
        "2.0.0",
        vec![version_entry(
            "2.0.0",
            "https://cdn.test/night.zip",
            &sha256_hex(&zip),
            "Dark theme.",
        )],
    );
    let f = setup(vec![], vec![entry], vec![("cdn.test/night.zip", zip)]);
    let report = f.market.install(THEME_ID, None).unwrap();
    assert_eq!(report.kind, ExtensionKind::Theme);
    assert!(f
        .dir
        .join("themes")
        .join(THEME_ID)
        .join("theme.css")
        .is_file());
    let installed = f.market.installed().unwrap();
    assert_eq!(installed[0].state, "installed");
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn incompatible_versions_refused() {
    let manifest = plugin_manifest(PLUGIN_ID, "9.0.0", &["storage:cache"], &[]);
    let zip = zip_of(&[
        ("manifest.json", manifest.as_bytes()),
        ("plugin.wasm", &fixture_wasm()),
    ]);
    let mut entry = plugin_entry(
        PLUGIN_ID,
        &["storage:cache"],
        "9.0.0",
        vec![version_entry(
            "9.0.0",
            "https://cdn.test/future.zip",
            &sha256_hex(&zip),
            "Future.",
        )],
    );
    entry["min_sonora_version"] = serde_json::json!("99.0.0");
    let f = setup(vec![entry], vec![], vec![("cdn.test/future.zip", zip)]);
    let err = f.market.install(PLUGIN_ID, None).unwrap_err().to_string();
    assert!(
        err.contains("NoCompatibleVersion") || err.contains("no compatible version"),
        "unexpected: {err}"
    );
    assert!(!f.dir.join("plugins").join(PLUGIN_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn untrusted_metadata_rejected() {
    let mut bad: Vec<serde_json::Value> = Vec::new();
    // Bad id.
    let (entry, _) = plugin_v1();
    let mut e1 = entry.clone();
    e1["id"] = serde_json::json!("../evil");
    bad.push(e1);
    let f = setup(bad, vec![], vec![]);
    assert!(f.market.refresh().is_err());
    let _ = std::fs::remove_dir_all(&f.dir);

    // Non-https download URL.
    let (mut entry2, _) = plugin_v1();
    entry2["versions"][0]["download_url"] = serde_json::json!("http://cdn.test/p.zip");
    let f = setup(vec![entry2], vec![], vec![]);
    assert!(f.market.refresh().is_err());
    let _ = std::fs::remove_dir_all(&f.dir);

    // latest_version pointing nowhere.
    let (mut entry3, _) = plugin_v1();
    entry3["latest_version"] = serde_json::json!("3.0.0");
    let f = setup(vec![entry3], vec![], vec![]);
    assert!(f.market.refresh().is_err());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn offline_catalog_serves_cache() {
    let (entry, zip) = plugin_v1();
    let f = setup(
        vec![entry],
        vec![],
        vec![("org.example.test-1.0.0.zip", zip)],
    );
    let fresh = f.market.refresh().unwrap();
    assert!(!fresh.offline);
    // Break the network: catalog must still serve, flagged offline.
    f.fake
        .route("plugins.json", FakeOutcome::Io("down".to_string()));
    let cached = f.market.catalog().unwrap();
    assert!(cached.offline);
    assert_eq!(cached.plugins.len(), 1);
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn paths_helpers_exposed() {
    let dir = test_dir("paths");
    let paths = MarketplacePaths::new(&dir);
    paths.ensure_dirs().unwrap();
    assert!(paths.plugins_dir.is_dir());
    assert!(paths.themes_dir.is_dir());
    let _ = std::fs::remove_dir_all(&dir);
}

fn theme_zip(id: &str, version: &str, css: Option<&[u8]>) -> Vec<u8> {
    let mut files: Vec<(&str, &[u8])> = Vec::new();
    let manifest = theme_manifest(id, version);
    files.push(("theme.json", manifest.as_bytes()));
    if let Some(css) = css {
        files.push(("theme.css", css));
    }
    zip_of(&files)
}

fn theme_setup(id: &str, version: &str, css: Option<Vec<u8>>) -> (Fixture, Vec<u8>) {
    let zip = theme_zip(id, version, css.as_deref());
    let entry = theme_entry(
        id,
        version,
        vec![version_entry(
            version,
            &format!("https://cdn.test/{id}-{version}.zip"),
            &sha256_hex(&zip),
            "Theme.",
        )],
    );
    let key = format!("{id}-{version}.zip");
    let f = setup(vec![], vec![entry], vec![(&*key.leak(), zip.clone())]);
    (f, zip)
}

#[test]
fn theme_install_registers_definition_and_css() {
    let (f, _) = theme_setup(THEME_ID, "2.0.0", Some(b":root { --x: #fff; }".to_vec()));
    let report = f.market.install(THEME_ID, None).unwrap();
    assert_eq!(report.kind, ExtensionKind::Theme);

    let themes = f.market.installed_themes().unwrap();
    assert_eq!(themes.len(), 1);
    assert_eq!(themes[0].id, THEME_ID);
    assert_eq!(themes[0].mode, "dark");
    assert!(themes[0].definition.tokens.contains_key("--bg-base"));
    assert!(themes[0].has_css);
    assert!(!themes[0].active);

    let def = f.market.theme_definition(THEME_ID).unwrap();
    assert_eq!(def.name, "Night Theme");
    let css = f.market.theme_css(THEME_ID).unwrap();
    assert_eq!(css.as_deref(), Some(":root { --x: #fff; }"));

    let installed = f.market.installed().unwrap();
    assert_eq!(installed[0].state, "installed");
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn theme_without_css_installs_and_serves_none() {
    let (f, _) = theme_setup(THEME_ID, "2.0.0", None);
    f.market.install(THEME_ID, None).unwrap();
    let themes = f.market.installed_themes().unwrap();
    assert!(!themes[0].has_css);
    assert_eq!(f.market.theme_css(THEME_ID).unwrap(), None);
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn malformed_theme_rejected_and_skipped_gracefully() {
    // Install refuses a package whose theme.json lacks required tokens.
    let bad_manifest = format!(
        "{{\"id\":\"{THEME_ID}\",\"name\":\"Bad\",\"version\":\"9.9.9\",\"author\":\"T\",\"mode\":\"dark\",\"tokens\":{{\"--bg-base\":\"#000\"}}}}"
    );
    let bad_zip = zip_of(&[("theme.json", bad_manifest.as_bytes())]);
    let entry = theme_entry(
        THEME_ID,
        "9.9.9",
        vec![version_entry(
            "9.9.9",
            "https://cdn.test/bad.zip",
            &sha256_hex(&bad_zip),
            "Bad.",
        )],
    );
    let f = setup(vec![], vec![entry], vec![("cdn.test/bad.zip", bad_zip)]);
    assert!(f.market.install(THEME_ID, None).is_err());
    assert!(!f.dir.join("themes").join(THEME_ID).exists());

    // Corrupting an installed theme.json degrades gracefully: discovery
    // skips it, installed() flags it invalid, nothing panics.
    let (f, _) = theme_setup(THEME_ID, "2.0.0", None);
    f.market.install(THEME_ID, None).unwrap();
    std::fs::write(
        f.dir.join("themes").join(THEME_ID).join("theme.json"),
        "{broken",
    )
    .unwrap();
    assert!(f.market.installed_themes().unwrap().is_empty());
    let installed = f.market.installed().unwrap();
    assert_eq!(installed[0].state, "invalid");
    assert!(f.market.theme_definition(THEME_ID).is_err());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn theme_css_with_remote_fetch_rejected() {
    let (f, _) = theme_setup(
        THEME_ID,
        "2.0.0",
        Some(b".a { background: url(https://evil.test/a.png); }".to_vec()),
    );
    let err = f.market.install(THEME_ID, None).unwrap_err().to_string();
    assert!(err.contains("theme.css"), "unexpected: {err}");
    assert!(!f.dir.join("themes").join(THEME_ID).exists());
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn active_theme_persists_and_resets_on_uninstall() {
    let (f, _) = theme_setup(THEME_ID, "2.0.0", None);
    f.market.install(THEME_ID, None).unwrap();

    assert_eq!(f.market.active_theme().unwrap(), None);
    assert!(f.market.set_active_theme(Some("org.sonora.ghost")).is_err());
    f.market.set_active_theme(Some(THEME_ID)).unwrap();
    assert_eq!(f.market.active_theme().unwrap().as_deref(), Some(THEME_ID));

    // Persistence survives a fresh Marketplace over the same data dir.
    let host2 = Arc::new(PluginHost::new().unwrap());
    let market2 = Marketplace::new(&f.dir, host2, Some(BASE.to_string()), None).unwrap();
    assert_eq!(market2.active_theme().unwrap().as_deref(), Some(THEME_ID));
    let installed = market2.installed().unwrap();
    assert_eq!(installed[0].state, "active");

    // Uninstalling the active theme clears the dangling selection.
    market2.uninstall(THEME_ID).unwrap();
    assert_eq!(market2.active_theme().unwrap(), None);
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn theme_update_then_rollback_restores_tokens() {
    let zip_v1 = theme_zip(THEME_ID, "1.0.0", None);
    let manifest_v2 = theme_manifest(THEME_ID, "1.1.0").replace("#112233", "#445566");
    let zip_v2 = zip_of(&[("theme.json", manifest_v2.as_bytes())]);
    let entry = theme_entry(
        THEME_ID,
        "1.1.0",
        vec![
            version_entry(
                "1.0.0",
                "https://cdn.test/t-1.0.0.zip",
                &sha256_hex(&zip_v1),
                "First.",
            ),
            version_entry(
                "1.1.0",
                "https://cdn.test/t-1.1.0.zip",
                &sha256_hex(&zip_v2),
                "Second.",
            ),
        ],
    );
    let f = setup(
        vec![],
        vec![entry],
        vec![("t-1.0.0.zip", zip_v1), ("t-1.1.0.zip", zip_v2)],
    );

    f.market.install(THEME_ID, Some("1.0.0")).unwrap();
    f.market.set_active_theme(Some(THEME_ID)).unwrap();
    f.market.update(THEME_ID).unwrap();
    assert_eq!(
        f.market.theme_definition(THEME_ID).unwrap().tokens["--bg-base"],
        "#445566"
    );
    // Active selection survives the update (same id).
    assert_eq!(f.market.active_theme().unwrap().as_deref(), Some(THEME_ID));

    let rb = f.market.rollback(THEME_ID, None).unwrap();
    assert_eq!(rb.restored_version, "1.0.0");
    assert_eq!(
        f.market.theme_definition(THEME_ID).unwrap().tokens["--bg-base"],
        "#112233"
    );
    assert_eq!(f.market.active_theme().unwrap().as_deref(), Some(THEME_ID));
    let _ = std::fs::remove_dir_all(&f.dir);
}

#[test]
fn sample_community_registry_validates() {
    use sonora_registry::{RegistryIndex, RegistryPlugin, RegistryTheme};

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../community-registry");
    let plugins_text =
        std::fs::read_to_string(root.join("v1/plugins.json")).expect("sample plugins.json");
    let themes_text =
        std::fs::read_to_string(root.join("v1/themes.json")).expect("sample themes.json");
    let plugins: Vec<RegistryPlugin> = RegistryIndex::plugins_from_json(&plugins_text).unwrap();
    let themes: Vec<RegistryTheme> = RegistryIndex::themes_from_json(&themes_text).unwrap();
    assert!(plugins.iter().any(|p| p.id() == "org.sonora.lrclib"));
    assert!(themes
        .iter()
        .any(|t| t.id() == "org.sonora.theme.neon_night"));

    // Every advertised artifact must exist on disk with a matching digest.
    let mut catalog: Vec<(&str, &[sonora_registry::RegistryVersion])> = Vec::new();
    for p in &plugins {
        catalog.push((p.id(), p.versions()));
    }
    for t in &themes {
        catalog.push((t.id(), t.versions()));
    }
    for (id, versions) in catalog {
        let _ = id;
        for v in versions {
            let zip_name = v
                .download_url
                .rsplit('/')
                .next()
                .expect("download url has a file name");
            let bytes = std::fs::read(root.join("packages").join(zip_name))
                .unwrap_or_else(|_| panic!("missing package {zip_name}"));
            let actual: String = {
                use sha2::Digest as _;
                sha2::Sha256::digest(&bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect()
            };
            assert_eq!(actual, v.sha256, "digest mismatch for {zip_name}");
        }
    }
}
#[test]
fn first_party_retro_theme_passes_security_validation() {
    use sonora_registry::theme::{validate_theme_css, validate_theme_definition};
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../themes/sonora-retro");
    let text = std::fs::read_to_string(dir.join("theme.json")).expect("retro theme.json");
    let def = validate_theme_definition(&text).expect("retro theme must validate");
    assert_eq!(def.id, "org.sonora.theme.retro");
    assert_eq!(def.name, "Sonora Retro");
    assert_eq!(def.mode, sonora_registry::theme::ThemeMode::Dark);
    assert_eq!(
        def.tokens.len(),
        sonora_registry::theme::REQUIRED_THEME_TOKENS.len()
    );
    // Visually distinctive: amber accent on near-black, not the neon default.
    assert_eq!(def.tokens["--accent-primary"], "#ffb000");
    assert_eq!(def.tokens["--bg-base"], "#14100a");

    let css = std::fs::read(dir.join("theme.css")).expect("retro theme.css");
    validate_theme_css(&def.id, &css).expect("retro theme.css must pass the guard");
    let css_text = String::from_utf8(css).unwrap();
    assert!(css_text.contains("repeating-linear-gradient"));
    assert!(!css_text.to_ascii_lowercase().contains("url("));
}

#[test]
fn sample_registry_packages_install_cleanly_end_to_end() {
    use sonora_plugin::{FakeNetworkTransport, FakeOutcome, PluginHost, PluginState};
    use sonora_registry::Marketplace;
    use std::sync::Arc;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../community-registry");
    let dir = test_dir("sample-install");
    let fake = Arc::new(FakeNetworkTransport::new());
    // Serve the real registry documents and the real package bytes.
    fake.route(
        "v1/plugins.json",
        FakeOutcome::Status(
            200,
            std::fs::read(root.join("v1/plugins.json")).expect("sample plugins.json"),
        ),
    );
    fake.route(
        "v1/themes.json",
        FakeOutcome::Status(
            200,
            std::fs::read(root.join("v1/themes.json")).expect("sample themes.json"),
        ),
    );
    for zip in [
        "org.sonora.lrclib-1.0.0.zip",
        "org.sonora.spectrum_plus-1.0.0.zip",
        "org.sonora.lyrics_genius-1.0.0.zip",
        "org.sonora.theme.neon_night-1.0.0.zip",
        "org.sonora.theme.retro-1.0.0.zip",
    ] {
        let bytes = std::fs::read(root.join("packages").join(zip)).expect("sample package");
        fake.route(zip, FakeOutcome::Status(200, bytes));
    }

    let host = Arc::new(PluginHost::new().unwrap());
    let market = Marketplace::new(
        &dir,
        Arc::clone(&host),
        Some("https://registry.test/v1".to_string()),
        Some(fake),
    )
    .unwrap();

    // Install every sample package; plugins must reach Running.
    for id in [
        "org.sonora.lrclib",
        "org.sonora.spectrum_plus",
        "org.sonora.lyrics_genius",
    ] {
        let report = market.install(id, None).expect("install plugin");
        assert!(report.fresh_install);
        assert!(
            report.started,
            "{id} did not start: {:?}",
            report.start_error
        );
        assert_eq!(host.state(id), Some(PluginState::Running));
    }
    for id in ["org.sonora.theme.neon_night", "org.sonora.theme.retro"] {
        let report = market.install(id, None).expect("install theme");
        assert!(report.fresh_install);
        assert!(dir.join("themes").join(id).join("theme.json").is_file());
    }

    let installed = market.installed().unwrap();
    assert_eq!(installed.len(), 5, "{installed:?}");
    assert!(market
        .installed_themes()
        .unwrap()
        .iter()
        .any(|t| t.id == "org.sonora.theme.retro"));

    // Reinstalling the same version refuses instead of overwriting silently.
    let err = market
        .install("org.sonora.theme.retro", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("already installed"), "unexpected: {err}");

    let _ = std::fs::remove_dir_all(&dir);
}
