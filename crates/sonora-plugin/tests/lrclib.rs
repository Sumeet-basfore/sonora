//! Integration tests for the real LRCLIB plugin (`plugins/lyrics-lrclib`).
//!
//! Hermetic by default: [`FakeNetworkTransport`] scripts LRCLIB answers, so
//! every case below runs without network. One `#[ignore]`d test exercises
//! the live API and is run explicitly for release verification.

use sonora_plugin::{
    Capability, FakeNetworkTransport, FakeOutcome, LyricsQueryDto, PluginHost, PluginManifest,
    PluginState,
};
use std::path::PathBuf;
use std::sync::Arc;

const PLUGIN_ID: &str = "org.sonora.lrclib";

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/lyrics-lrclib")
}

fn real_manifest() -> PluginManifest {
    PluginManifest::from_file(&plugin_dir().join("manifest.json")).expect("real manifest")
}

fn query(title: &str) -> LyricsQueryDto {
    LyricsQueryDto {
        title: title.to_string(),
        artist: Some("No Such Artist".to_string()),
        album: None,
        duration_ms: Some(180_000),
    }
}

fn synced_body() -> Vec<u8> {
    br#"{"trackName":"Xyzzy Fake Qqqq","artistName":"No Such Artist","syncedLyrics":"[00:01.00] Fake synced line\n[00:05.00] Second line\n","plainLyrics":"Fake synced line\nSecond line","instrumental":false}"#.to_vec()
}

fn start_with_fake(routes: Vec<(&str, FakeOutcome)>) -> (PluginHost, Arc<FakeNetworkTransport>) {
    let fake = Arc::new(FakeNetworkTransport::new());
    for (key, outcome) in routes {
        fake.route(key, outcome);
    }
    let manifest = real_manifest();
    let wasm = std::fs::read(plugin_dir().join("plugin.wasm")).expect("built plugin.wasm");
    let host = PluginHost::new().expect("engine");
    host.set_network_transport(fake.clone());
    host.register(manifest, plugin_dir()).expect("register");
    host.load_bytes(PLUGIN_ID, &wasm).expect("load");
    host.start(PLUGIN_ID).expect("start");
    assert_eq!(host.state(PLUGIN_ID), Some(PluginState::Running));
    (host, fake)
}

#[test]
fn real_manifest_validates_with_expected_shape() {
    let m = real_manifest();
    assert_eq!(m.id, PLUGIN_ID);
    assert_eq!(m.api_version, 1);
    assert_eq!(m.entry, "plugin.wasm");
    assert!(m.capabilities.contains(Capability::LyricsProvider));
    assert!(m.capabilities.contains(Capability::NetworkFetch));
    assert_eq!(m.allowed_domains, vec!["https://lrclib.net".to_string()]);
    assert!(m.is_url_allowed("https://lrclib.net/api/get?track_name=x"));
    assert!(!m.is_url_allowed("https://lrclib.com/api/get"));
    assert!(!m.is_url_allowed("http://lrclib.net/api/get"));
}

#[test]
fn real_plugin_dir_discovers() {
    let plugins_root = plugin_dir().parent().expect("plugins root").to_path_buf();
    let host = PluginHost::new().unwrap();
    let found = host.discover(&plugins_root);
    assert!(
        found.contains(&PLUGIN_ID.to_string()),
        "discovered: {found:?}"
    );
    assert_eq!(host.state(PLUGIN_ID), Some(PluginState::Validated));
}

#[test]
fn resolves_synced_lyrics_through_fake_transport() {
    let (host, fake) = start_with_fake(vec![(
        "lrclib.net",
        FakeOutcome::Status(200, synced_body()),
    )]);
    let answer = host
        .fetch_lyrics(PLUGIN_ID, &query("Xyzzy Fake Qqqq"))
        .unwrap()
        .unwrap();
    assert_eq!(answer.format, "lrc");
    assert!(answer.content.contains("Fake synced line"));
    assert_eq!(answer.attribution.as_deref(), Some("LRCLIB (lrclib.net)"));

    // The plugin built a well-formed, encoded LRCLIB query — and only ever
    // talked to the allow-listed origin.
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    let url = &calls[0];
    assert!(
        url.starts_with("https://lrclib.net/api/get?track_name="),
        "{url}"
    );
    assert!(url.contains("track_name=Xyzzy%20Fake%20Qqqq"), "{url}");
    assert!(url.contains("artist_name=No%20Such%20Artist"), "{url}");
    assert!(url.contains("duration=180"), "{url}");
}

#[test]
fn falls_back_to_plain_lyrics() {
    let body =
        br#"{"syncedLyrics":null,"plainLyrics":"Just prose here","instrumental":false}"#.to_vec();
    let (host, _) = start_with_fake(vec![("lrclib.net", FakeOutcome::Status(200, body))]);
    let answer = host
        .fetch_lyrics(PLUGIN_ID, &query("Plain Song"))
        .unwrap()
        .unwrap();
    assert_eq!(answer.format, "plain");
    assert!(answer.content.contains("Just prose"));
}

#[test]
fn graceful_miss_on_all_failure_modes() {
    let cases: Vec<(&str, FakeOutcome)> = vec![
        ("missing", FakeOutcome::Status(404, vec![])),
        ("limited", FakeOutcome::Status(429, vec![])),
        ("broken", FakeOutcome::Status(500, vec![])),
        ("slow", FakeOutcome::Timeout),
        ("io", FakeOutcome::Io("reset".to_string())),
        ("garbage", FakeOutcome::Status(200, b"not json".to_vec())),
        (
            "empty",
            FakeOutcome::Status(200, br#"{"syncedLyrics":"  "}"#.to_vec()),
        ),
        (
            "karaoke-none",
            FakeOutcome::Status(200, br#"{"instrumental":true}"#.to_vec()),
        ),
    ];
    for (key, outcome) in cases {
        let (host, _) = start_with_fake(vec![(key, outcome)]);
        let title = format!("Song {key}");
        // Route key must appear in the generated URL: embed it in the title.
        let q = query(&title);
        assert!(
            host.fetch_lyrics(PLUGIN_ID, &q).unwrap().is_none(),
            "case {key} should miss"
        );
        // A miss never crashes the plugin.
        assert_eq!(host.state(PLUGIN_ID), Some(PluginState::Running));
    }
}

#[test]
fn loading_without_network_capability_fails_closed() {
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin_dir().join("manifest.json")).unwrap())
            .unwrap();
    value["capabilities"] = serde_json::json!(["lyrics:provider"]);
    value["allowed_domains"] = serde_json::json!([]);
    let manifest = PluginManifest::from_json(&value.to_string()).unwrap();

    let wasm = std::fs::read(plugin_dir().join("plugin.wasm")).unwrap();
    let host = PluginHost::new().unwrap();
    host.register(manifest, plugin_dir()).unwrap();
    let err = host.load_bytes(PLUGIN_ID, &wasm).unwrap_err().to_string();
    assert!(err.contains("network:fetch"), "unexpected error: {err}");
}

#[test]
fn tampered_manifest_with_native_capability_rejected() {
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin_dir().join("manifest.json")).unwrap())
            .unwrap();
    value["capabilities"] = serde_json::json!(["lyrics:provider", "filesystem:write"]);
    assert!(PluginManifest::from_json(&value.to_string()).is_err());
}

/// Live verification against the real LRCLIB API. Ignored by default
/// (network-dependent); run explicitly: `cargo test -p sonora-plugin live_`.
#[test]
#[ignore]
fn live_resolves_real_track() {
    let manifest = real_manifest();
    let wasm = std::fs::read(plugin_dir().join("plugin.wasm")).expect("built plugin.wasm");
    let host = PluginHost::new().expect("engine");
    host.register(manifest, plugin_dir()).expect("register");
    host.load_bytes(PLUGIN_ID, &wasm).expect("load");
    host.start(PLUGIN_ID).expect("start");

    let q = LyricsQueryDto {
        title: "Get Lucky".to_string(),
        artist: Some("Daft Punk".to_string()),
        album: Some("Random Access Memories".to_string()),
        duration_ms: Some(369_000),
    };
    let answer = host
        .fetch_lyrics(PLUGIN_ID, &q)
        .expect("fetch")
        .expect("LRCLIB should know Get Lucky");
    assert_eq!(answer.format, "lrc");
    assert!(!answer.content.trim().is_empty());
}
