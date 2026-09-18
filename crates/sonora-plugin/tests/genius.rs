//! Integration tests for the Genius lyrics provider (`plugins/lyrics-genius`).
//!
//! Hermetic by default: [`FakeNetworkTransport`] scripts the Genius search
//! API and song pages, so every case below runs without network. One
//! `#[ignore]`d test exercises the live endpoints and is run explicitly.

use sonora_plugin::{
    Capability, FakeNetworkTransport, FakeOutcome, LyricsQueryDto, PluginHost, PluginManifest,
    PluginState,
};
use std::path::PathBuf;
use std::sync::Arc;

const PLUGIN_ID: &str = "org.sonora.lyrics_genius";

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/lyrics-genius")
}

fn real_manifest() -> PluginManifest {
    PluginManifest::from_file(&plugin_dir().join("manifest.json")).expect("real manifest")
}

fn query(title: &str) -> LyricsQueryDto {
    LyricsQueryDto {
        title: title.to_string(),
        artist: Some("Kendrick Lamar".to_string()),
        album: None,
        duration_ms: Some(234_000),
    }
}

const SONG_URL: &str = "https://genius.com/Kendrick-lamar-humble-lyrics";

fn search_body(song_url: &str) -> Vec<u8> {
    format!(
        r#"{{"response":{{"sections":[{{"type":"song","hits":[{{"result":{{"url":"{song_url}","title":"HUMBLE.","artist_names":"Kendrick Lamar"}}}}]}}]}}}}"#
    )
    .into_bytes()
}

fn song_page() -> Vec<u8> {
    br#"<html><body><div class="ad">ad</div>
<div data-lyrics-container="true">[Intro]<br/>Nobody pray for me<br/></div>
<div data-lyrics-container="true">Been hustlin&rsquo; all day<br/></div>
</body></html>"#
        .to_vec()
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
    assert_eq!(m.allowed_domains, vec!["https://genius.com".to_string()]);
    assert!(m.is_url_allowed("https://genius.com/api/search/multi?per_page=5&q=x"));
    assert!(m.is_url_allowed("https://genius.com/Kendrick-lamar-humble-lyrics"));
    // Sub-domains of an allow-listed origin inherit access (host policy).
    assert!(m.is_url_allowed("https://api.genius.com/search"));
    assert!(!m.is_url_allowed("https://notgenius.com/"));
    assert!(!m.is_url_allowed("https://genius.com.evil.test/"));
    assert!(!m.is_url_allowed("http://genius.com/x"));
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
fn resolves_lyrics_through_fake_transport() {
    let (host, fake) = start_with_fake(vec![
        (
            "api/search/multi",
            FakeOutcome::Status(200, search_body(SONG_URL)),
        ),
        ("humble-lyrics", FakeOutcome::Status(200, song_page())),
    ]);
    let answer = host
        .fetch_lyrics(PLUGIN_ID, &query("HUMBLE."))
        .unwrap()
        .unwrap();
    assert_eq!(answer.format, "plain");
    assert!(
        answer.content.contains("Nobody pray for me"),
        "{}",
        answer.content
    );
    assert!(
        answer.content.contains("hustlin’ all day"),
        "{}",
        answer.content
    );
    assert!(!answer.content.contains("ad"), "{}", answer.content);
    let attribution = answer.attribution.expect("attribution");
    assert!(attribution.contains("genius.com"), "{attribution}");
    assert!(attribution.contains("humble-lyrics"), "{attribution}");

    // Both round-trips stayed on the allow-listed origin with a well-formed
    // search URL.
    let calls = fake.calls();
    assert_eq!(calls.len(), 2);
    assert!(
        calls[0].starts_with("https://genius.com/api/search/multi?per_page=5&q="),
        "{}",
        calls[0]
    );
    assert!(
        calls[0].contains("Kendrick%20Lamar%20HUMBLE."),
        "{}",
        calls[0]
    );
    assert_eq!(calls[1], SONG_URL);
    assert!(calls.iter().all(|u| u.starts_with("https://genius.com/")));
}

#[test]
fn graceful_miss_on_all_failure_modes() {
    let cases: Vec<(&str, Vec<(&str, FakeOutcome)>)> = vec![
        (
            "search-404",
            vec![("api/search/multi", FakeOutcome::Status(404, vec![]))],
        ),
        (
            "search-timeout",
            vec![("api/search/multi", FakeOutcome::Timeout)],
        ),
        (
            "search-garbage",
            vec![(
                "api/search/multi",
                FakeOutcome::Status(200, b"nope".to_vec()),
            )],
        ),
        (
            "no-song",
            vec![(
                "api/search/multi",
                FakeOutcome::Status(
                    200,
                    br#"{"response":{"sections":[{"type":"artist","hits":[]}]}}"#.to_vec(),
                ),
            )],
        ),
        (
            "page-404",
            vec![
                (
                    "api/search/multi",
                    FakeOutcome::Status(200, search_body(SONG_URL)),
                ),
                ("humble-lyrics", FakeOutcome::Status(404, vec![])),
            ],
        ),
        (
            "page-changed",
            vec![
                (
                    "api/search/multi",
                    FakeOutcome::Status(200, search_body(SONG_URL)),
                ),
                (
                    "humble-lyrics",
                    FakeOutcome::Status(200, b"<div>redesigned, no containers</div>".to_vec()),
                ),
            ],
        ),
        (
            "page-empty",
            vec![
                (
                    "api/search/multi",
                    FakeOutcome::Status(200, search_body(SONG_URL)),
                ),
                (
                    "humble-lyrics",
                    FakeOutcome::Status(
                        200,
                        br#"<div data-lyrics-container="true">x</div>"#.to_vec(),
                    ),
                ),
            ],
        ),
    ];
    for (name, routes) in cases {
        let (host, _) = start_with_fake(routes);
        assert!(
            host.fetch_lyrics(PLUGIN_ID, &query("HUMBLE."))
                .unwrap()
                .is_none(),
            "case {name} should miss"
        );
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
    let err = match host.load_bytes(PLUGIN_ID, &wasm) {
        Ok(_) => panic!("load without network:fetch should fail"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("network:fetch"), "unexpected error: {err}");
}

#[test]
fn large_song_page_does_not_trap() {
    // Realistic Genius song page size (~400KB) with the lyrics near the end.
    let mut page = String::from("<html><body>");
    page.push_str(&"x".repeat(380 * 1024));
    page.push_str(r#"<div data-lyrics-container="true">First real line here yes<br/>Second real line here yes<br/></div>"#);
    page.push_str("</body></html>");
    let (host, _) = start_with_fake(vec![
        (
            "api/search/multi",
            FakeOutcome::Status(200, search_body(SONG_URL)),
        ),
        ("humble-lyrics", FakeOutcome::Status(200, page.into_bytes())),
    ]);
    let answer = host.fetch_lyrics(PLUGIN_ID, &query("HUMBLE."));
    assert!(answer.is_ok(), "large page trapped: {answer:?}");
}

/// Live verification against the real Genius endpoints. Ignored by default
/// (network- and bot-protection-dependent); run explicitly.
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
        title: "HUMBLE.".to_string(),
        artist: Some("Kendrick Lamar".to_string()),
        album: None,
        duration_ms: Some(234_000),
    };
    let answer = host
        .fetch_lyrics(PLUGIN_ID, &q)
        .expect("fetch")
        .expect("Genius should know HUMBLE.");
    assert_eq!(answer.format, "plain");
    assert!(!answer.content.trim().is_empty());
}
