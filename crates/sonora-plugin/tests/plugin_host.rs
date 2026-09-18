//! Integration tests for the Sonora plugin foundation:
//! manifest validation, capability enforcement, lifecycle, malformed modules,
//! host-boundary probes (cache/net/library/tap), and crash isolation.
//!
//! Test WASM modules are built inline from WAT via the `wat` crate — no
//! external toolchain required.

use sonora_plugin::{
    Capability, FakeNetworkTransport, FakeOutcome, LyricsQueryDto, PluginHost, PluginManifest,
    PluginState,
};
use std::path::PathBuf;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// WAT builders
// ---------------------------------------------------------------------------

fn wat_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b'\n' => out.push_str("\\n"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{b:02x}")),
        }
    }
    out
}

const TEST_LRC: &str = "[00:01.00] Probe line one\n[00:05.00] Probe line two\n";

fn answer_json() -> String {
    serde_json::json!({"format": "lrc", "content": TEST_LRC}).to_string()
}

/// Full lyrics-provider module. `fetch_body` must leave an i32 status
/// (1 = hit with the static answer, 0 = miss) on the stack.
fn lyrics_module(imports: &str, fetch_body: &str) -> String {
    let answer = answer_json();
    format!(
        r#"(module
  {imports}
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 4096))
  (func $alloc (export "alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $ptr))
  (data (i32.const 64) "{esc}")
  (global $res_ptr (mut i32) (i32.const 0))
  (global $res_len (mut i32) (i32.const 0))
  (func (export "plugin_api_version") (result i32) (i32.const 1))
  (func (export "plugin_init") (result i32) (i32.const 0))
  (func (export "plugin_shutdown") (result i32) (i32.const 0))
  (func (export "lyrics_fetch") (param $ptr i32) (param $len i32) (result i32)
    {fetch_body})
  (func (export "lyrics_result_ptr") (result i32) (global.get $res_ptr))
  (func (export "lyrics_result_len") (result i32) (global.get $res_len))
)"#,
        esc = wat_escape(&answer)
    )
}

fn hit_body() -> String {
    let len = answer_json().len();
    format!(
        "(global.set $res_ptr (i32.const 64)) (global.set $res_len (i32.const {len})) (i32.const 1)"
    )
}

fn miss_body() -> String {
    "(i32.const 0)".to_string()
}

// ---------------------------------------------------------------------------
// Manifest / host helpers
// ---------------------------------------------------------------------------

fn manifest_json_domains(id: &str, caps: &[&str], domains: &[&str]) -> String {
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
        r#"{{"id":"{id}","name":"Test Plugin","version":"1.2.3","author":"Tester","description":"Integration test plugin.","api_version":1,"capabilities":[{caps_json}],"entry":"plugin.wasm","allowed_domains":[{domains_json}]}}"#
    )
}

fn manifest_json(id: &str, caps: &[&str]) -> String {
    manifest_json_domains(id, caps, &[])
}

fn start_plugin(id: &str, caps: &[&str], domains: &[&str], wat_src: &str) -> PluginHost {
    start_plugin_with_transport(id, caps, domains, wat_src, None)
}

fn start_plugin_with_transport(
    id: &str,
    caps: &[&str],
    domains: &[&str],
    wat_src: &str,
    transport: Option<Arc<dyn sonora_plugin::NetworkTransport>>,
) -> PluginHost {
    let wasm = wat::parse_str(wat_src).expect("valid test WAT");
    let manifest = PluginManifest::from_json(&manifest_json_domains(id, caps, domains))
        .expect("valid manifest");
    let host = PluginHost::new().expect("engine");
    if let Some(t) = transport {
        host.set_network_transport(t);
    }
    host.register(manifest, PathBuf::from("test-plugins"))
        .expect("register");
    host.load_bytes(id, &wasm).expect("load");
    host.start(id).expect("start");
    host
}

fn query() -> LyricsQueryDto {
    LyricsQueryDto {
        title: "Probe Song".to_string(),
        artist: Some("Probe Artist".to_string()),
        album: None,
        duration_ms: Some(200_000),
    }
}

// ---------------------------------------------------------------------------
// Manifest validation
// ---------------------------------------------------------------------------

#[test]
fn accepts_valid_manifest_with_all_capabilities() {
    let caps = [
        "library:read",
        "metadata:read",
        "lyrics:provider",
        "visualizer:tap",
        "ui:widget",
        "storage:cache",
    ];
    let m = PluginManifest::from_json(&manifest_json("org.example.full", &caps)).unwrap();
    assert_eq!(m.id, "org.example.full");
    assert_eq!(m.version.to_string(), "1.2.3");
    assert!(m.capabilities.contains(Capability::LyricsProvider));
    assert_eq!(m.entry, "plugin.wasm");
}

#[test]
fn accepts_network_manifest_with_allow_list() {
    let m = PluginManifest::from_json(&manifest_json_domains(
        "org.example.net",
        &["network:fetch"],
        &["https://api.example.com"],
    ))
    .unwrap();
    assert!(m.is_url_allowed("https://api.example.com/search?q=1"));
    assert!(m.is_url_allowed("https://sub.api.example.com/x"));
    assert!(!m.is_url_allowed("https://evil.example.org/"));
    assert!(!m.is_url_allowed("http://api.example.com/"));
}

#[test]
fn rejects_malformed_json() {
    assert!(PluginManifest::from_json("{not json").is_err());
    assert!(PluginManifest::from_json("{}").is_err());
}

#[test]
fn rejects_unknown_capability() {
    let json = manifest_json("org.example.bad", &["lyrics:provider", "filesystem:write"]);
    assert!(PluginManifest::from_json(&json).is_err());
}

#[test]
fn rejects_bad_ids() {
    for id in [
        "",
        "single",
        ".lead",
        "trail.",
        "UPPER.case",
        "has space.x",
        "a..b",
        "0..1",
    ] {
        let json = manifest_json(id, &["lyrics:provider"]);
        assert!(
            PluginManifest::from_json(&json).is_err(),
            "id accepted: {id}"
        );
    }
}

#[test]
fn rejects_bad_versions() {
    for v in ["", "1.0", "abc", "1.0.0.0.0.0"] {
        let json = manifest_json("org.example.v", &["lyrics:provider"]).replace("1.2.3", v);
        assert!(
            PluginManifest::from_json(&json).is_err(),
            "version accepted: {v}"
        );
    }
}

#[test]
fn rejects_wrong_api_version() {
    for api in [0, 2, 99] {
        let json = manifest_json("org.example.v", &["lyrics:provider"])
            .replace("\"api_version\":1", &format!("\"api_version\":{api}"));
        assert!(
            PluginManifest::from_json(&json).is_err(),
            "api accepted: {api}"
        );
    }
}

#[test]
fn rejects_network_without_domains() {
    let json = manifest_json("org.example.net", &["network:fetch"]);
    assert!(PluginManifest::from_json(&json).is_err());
}

#[test]
fn rejects_domains_without_network_capability() {
    let json = manifest_json_domains(
        "org.example.net",
        &["lyrics:provider"],
        &["https://api.example.com"],
    );
    assert!(PluginManifest::from_json(&json).is_err());
}

#[test]
fn rejects_non_https_domains() {
    for d in [
        "http://api.example.com",
        "api.example.com",
        "https://",
        "ftp://x.example.com",
    ] {
        let json = manifest_json_domains("org.example.net", &["network:fetch"], &[d]);
        assert!(
            PluginManifest::from_json(&json).is_err(),
            "domain accepted: {d}"
        );
    }
}

#[test]
fn rejects_bad_entry_paths() {
    for entry in [
        "",
        "../evil.wasm",
        "/abs/plugin.wasm",
        "sub/dir/p.wasm",
        "plugin.wasi",
        "plugin.js",
    ] {
        let json =
            manifest_json("org.example.e", &["lyrics:provider"]).replace("plugin.wasm", entry);
        // Note: replacing inside JSON may also hit other fields; entry is unique.
        assert!(
            PluginManifest::from_json(&json).is_err(),
            "entry accepted: {entry}"
        );
    }
}

#[test]
fn rejects_duplicate_and_empty_capabilities() {
    let json = manifest_json("org.example.d", &["lyrics:provider", "lyrics:provider"]);
    assert!(PluginManifest::from_json(&json).is_err());
    let json = manifest_json("org.example.d", &[]);
    assert!(PluginManifest::from_json(&json).is_err());
}

#[test]
fn accepts_object_author_and_rejects_empty_author() {
    let mut v: serde_json::Value =
        serde_json::from_str(&manifest_json("org.example.a", &["lyrics:provider"])).unwrap();
    v["author"] = serde_json::json!({"name": " Ada ", "url": "https://example.com"});
    assert!(PluginManifest::from_json(&v.to_string()).is_ok());
    v["author"] = serde_json::json!("  ");
    assert!(PluginManifest::from_json(&v.to_string()).is_err());
}

// ---------------------------------------------------------------------------
// Capability model
// ---------------------------------------------------------------------------

#[test]
fn capability_strings_round_trip() {
    for cap in Capability::ALL {
        assert_eq!(cap.to_string(), cap.as_str());
        assert_eq!(cap.as_str().parse::<Capability>().unwrap(), cap);
    }
    assert!("filesystem:write".parse::<Capability>().is_err());
}

#[test]
fn allowed_imports_follow_capabilities() {
    let none = manifest_json("org.example.n", &["lyrics:provider"]);
    let m = PluginManifest::from_json(&none).unwrap();
    assert!(m.capabilities.allowed_imports().contains("log"));
    assert!(!m.capabilities.allowed_imports().contains("cache_put"));

    let full = manifest_json_domains(
        "org.example.f",
        &[
            "storage:cache",
            "network:fetch",
            "library:read",
            "visualizer:tap",
        ],
        &["https://api.example.com"],
    );
    let m = PluginManifest::from_json(&full).unwrap();
    let allowed = m.capabilities.allowed_imports();
    for f in [
        "log",
        "cache_get",
        "cache_put",
        "net_fetch",
        "lib_stats",
        "tap_read",
    ] {
        assert!(allowed.contains(f), "missing {f}");
    }
}

#[test]
fn export_capability_gating() {
    let lyrics_only =
        PluginManifest::from_json(&manifest_json("org.example.l", &["lyrics:provider"])).unwrap();
    assert_eq!(
        lyrics_only
            .capabilities
            .missing_capability_for_exports(["lyrics_fetch"].into_iter()),
        None
    );
    assert_eq!(
        lyrics_only
            .capabilities
            .missing_capability_for_exports(["metadata_fetch"].into_iter()),
        Some(Capability::MetadataRead)
    );
    // Unknown exports are ignored (future-proofing).
    assert_eq!(
        lyrics_only
            .capabilities
            .missing_capability_for_exports(["some_future_hook"].into_iter()),
        None
    );
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[test]
fn full_lifecycle_with_events() {
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.life", &["lyrics:provider"]))
            .unwrap();
    let wasm = wat::parse_str(lyrics_module("", &hit_body())).unwrap();

    host.register(manifest, PathBuf::from("test-plugins"))
        .unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Validated));

    host.load_bytes("org.example.life", &wasm).unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Loaded));

    host.start("org.example.life").unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Running));

    let answer = host
        .fetch_lyrics("org.example.life", &query())
        .unwrap()
        .unwrap();
    assert_eq!(answer.format, "lrc");
    assert!(answer.content.contains("Probe line one"));

    host.stop("org.example.life").unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Stopped));

    host.start("org.example.life").unwrap();
    host.unload("org.example.life").unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Unloaded));

    // Reload after unload works.
    host.load_bytes("org.example.life", &wasm).unwrap();
    host.start("org.example.life").unwrap();
    assert_eq!(host.state("org.example.life"), Some(PluginState::Running));

    let events = host.drain_events();
    let names: Vec<&str> = events.iter().map(|e| e.to).collect();
    for expected in [
        "validated",
        "loaded",
        "running",
        "stopped",
        "running",
        "unloaded",
        "loaded",
        "running",
    ] {
        assert!(names.contains(&expected), "missing {expected} in {names:?}");
    }
}

#[test]
fn rejects_bad_transitions() {
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.bad", &["lyrics:provider"])).unwrap();
    let wasm = wat::parse_str(lyrics_module("", &hit_body())).unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();

    assert!(host.start("org.example.bad").is_err()); // not loaded
    assert!(host.stop("org.example.bad").is_err()); // not running
    assert!(host.fetch_lyrics("org.example.bad", &query()).is_err()); // not running

    host.load_bytes("org.example.bad", &wasm).unwrap();
    assert!(host.load_bytes("org.example.bad", &wasm).is_err()); // double load
    assert!(host.stop("org.example.bad").is_err()); // loaded but not running
}

#[test]
fn discover_accepts_good_and_skips_bad() {
    let dir = std::env::temp_dir().join(format!("sonora-discover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("good")).unwrap();
    std::fs::create_dir_all(dir.join("bad")).unwrap();
    std::fs::create_dir_all(dir.join("empty")).unwrap();
    std::fs::write(
        dir.join("good").join("manifest.json"),
        manifest_json("org.example.good", &["lyrics:provider"]),
    )
    .unwrap();
    std::fs::write(dir.join("bad").join("manifest.json"), "{broken").unwrap();

    let host = PluginHost::new().unwrap();
    let found = host.discover(&dir);
    assert_eq!(found, vec!["org.example.good".to_string()]);
    assert_eq!(host.state("org.example.good"), Some(PluginState::Validated));

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Malformed / hostile modules
// ---------------------------------------------------------------------------

#[test]
fn rejects_empty_and_garbage_bytes() {
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.mal", &["lyrics:provider"])).unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    assert!(host.load_bytes("org.example.mal", &[]).is_err());
    assert!(host
        .load_bytes("org.example.mal", b"definitely not wasm")
        .is_err());
    assert!(host
        .load_bytes("org.example.mal", b"\0asm\x01\0\0\0garbage")
        .is_err());
    // State never leaves validated on failed loads.
    assert_eq!(host.state("org.example.mal"), Some(PluginState::Validated));
}

#[test]
fn rejects_wasi_and_unknown_imports() {
    let wasi = r#"(module
      (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1)))"#;
    let unknown = r#"(module
      (import "sonora" "evil" (func (param i32) (result i32)))
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1)))"#;
    for (name, src) in [("wasi", wasi), ("unknown", unknown)] {
        let host = PluginHost::new().unwrap();
        let id = format!("org.example.{name}");
        let manifest =
            PluginManifest::from_json(&manifest_json(&id, &["lyrics:provider"])).unwrap();
        let wasm = wat::parse_str(src).unwrap();
        host.register(manifest, PathBuf::from("t")).unwrap();
        assert!(
            host.load_bytes(&id, &wasm).is_err(),
            "{name} import accepted"
        );
    }
}

#[test]
fn rejects_undeclared_import_and_export() {
    // Imports cache_put without declaring storage:cache.
    let cache_src = lyrics_module(
        r#"(import "sonora" "cache_put" (func $cache_put (param i32 i32 i32 i32) (result i32)))"#,
        &miss_body(),
    );
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.nodec", &["lyrics:provider"]))
            .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    let wasm = wat::parse_str(&cache_src).unwrap();
    let err = host
        .load_bytes("org.example.nodec", &wasm)
        .unwrap_err()
        .to_string();
    assert!(err.contains("storage:cache"), "unexpected error: {err}");

    // Exports lyrics_fetch without declaring lyrics:provider.
    let export_src = lyrics_module("", &hit_body());
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.noexp", &["storage:cache"])).unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    let wasm = wat::parse_str(&export_src).unwrap();
    let err = host
        .load_bytes("org.example.noexp", &wasm)
        .unwrap_err()
        .to_string();
    assert!(err.contains("lyrics:provider"), "unexpected error: {err}");
}

#[test]
fn rejects_missing_memory_alloc_and_version() {
    let no_mem = r#"(module (func (export "plugin_api_version") (result i32) (i32.const 1)))"#;
    // lyrics_fetch present but no alloc export.
    let no_alloc = r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1))
      (func (export "plugin_init") (result i32) (i32.const 0))
      (func (export "lyrics_fetch") (param i32 i32) (result i32) (i32.const 0))
      (func (export "lyrics_result_ptr") (result i32) (i32.const 0))
      (func (export "lyrics_result_len") (result i32) (i32.const 0)))"#;
    let no_version = r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_init") (result i32) (i32.const 0)))"#;
    let wrong_version = r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 2)))"#;
    for (name, src) in [
        ("no-memory", no_mem),
        ("no-alloc", no_alloc),
        ("no-version", no_version),
        ("wrong-version", wrong_version),
    ] {
        let host = PluginHost::new().unwrap();
        let id = format!("org.example.{}", name.replace('-', ""));
        let manifest =
            PluginManifest::from_json(&manifest_json(&id, &["lyrics:provider"])).unwrap();
        host.register(manifest, PathBuf::from("t")).unwrap();
        let wasm = wat::parse_str(src).unwrap();
        assert!(host.load_bytes(&id, &wasm).is_err(), "{name} accepted");
    }
}

// ---------------------------------------------------------------------------
// Host boundary probes
// ---------------------------------------------------------------------------

#[test]
fn cache_roundtrip_through_plugin() {
    let src = lyrics_module(
        r#"(import "sonora" "cache_put" (func $cache_put (param i32 i32 i32 i32) (result i32)))
          (import "sonora" "cache_get" (func $cache_get (param i32 i32 i32 i32) (result i32)))
          (data (i32.const 0) "k")
          (data (i32.const 8) "v")"#,
        r#"(local $ok i32)
          (local.set $ok (i32.const 1))
          (if (i32.ne (call $cache_put (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 1)) (i32.const 0))
            (then (local.set $ok (i32.const 0))))
          (if (i32.ne (call $cache_get (i32.const 0) (i32.const 1) (i32.const 16) (i32.const 64)) (i32.const 1))
            (then (local.set $ok (i32.const 0))))
          (if (i32.ne (i32.load8_u (i32.const 16)) (i32.const 118))
            (then (local.set $ok (i32.const 0))))
          (if (result i32) (local.get $ok)
            (then
              (global.set $res_ptr (i32.const 64))
              (global.set $res_len (i32.const ANSWER_LEN))
              (i32.const 1))
            (else (i32.const 0)))"#
            .replace("ANSWER_LEN", &answer_json().len().to_string())
            .as_str(),
    );
    let host = start_plugin(
        "org.example.cache",
        &["lyrics:provider", "storage:cache"],
        &[],
        &src,
    );
    let answer = host
        .fetch_lyrics("org.example.cache", &query())
        .unwrap()
        .unwrap();
    assert!(answer.content.contains("Probe line one"));
    // Cache persists across calls.
    let again = host
        .fetch_lyrics("org.example.cache", &query())
        .unwrap()
        .unwrap();
    assert!(again.content.contains("Probe line two"));
}

#[test]
fn cache_quota_is_enforced() {
    // Five 256 KiB puts under distinct keys: the fifth exceeds the 1 MiB quota.
    let src = lyrics_module(
        r#"(import "sonora" "cache_put" (func $cache_put (param i32 i32 i32 i32) (result i32)))"#,
        r#"(local $i i32) (local $buf i32) (local $r i32) (local $denied i32)
          (local.set $i (i32.const 0))
          (local.set $denied (i32.const 0))
          (loop $again
            (i32.store8 (i32.add (i32.const 8) (local.get $i)) (local.get $i))
            (local.set $buf (call $alloc (i32.const 262144)))
            (local.set $r (call $cache_put (i32.add (i32.const 8) (local.get $i)) (i32.const 1) (local.get $buf) (i32.const 262144)))
            (if (i32.and (i32.eq (local.get $i) (i32.const 4)) (i32.eq (local.get $r) (i32.const -1)))
              (then (local.set $denied (i32.const 1))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br_if $again (i32.lt_u (local.get $i) (i32.const 5))))
          (if (result i32) (local.get $denied)
            (then
              (global.set $res_ptr (i32.const 64))
              (global.set $res_len (i32.const ANSWER_LEN))
              (i32.const 1))
            (else (i32.const 0)))"#
            .replace("ANSWER_LEN", &answer_json().len().to_string())
            .as_str(),
    );
    let host = start_plugin(
        "org.example.quota",
        &["lyrics:provider", "storage:cache"],
        &[],
        &src,
    );
    let answer = host
        .fetch_lyrics("org.example.quota", &query())
        .unwrap()
        .unwrap();
    assert!(answer.content.contains("Probe line one"));
}

#[test]
fn network_allow_list_is_enforced() {
    let mk = |url: &str| {
        lyrics_module(
            &format!(
                "(import \"sonora\" \"net_fetch\" (func $net_fetch (param i32 i32 i32 i32) (result i32)))\n  (data (i32.const 256) \"{url}\")"
            ),
            &format!(
                r#"(if (result i32) (i32.ge_s (call $net_fetch (i32.const 256) (i32.const {ulen}) (i32.const 512) (i32.const 1024)) (i32.const 0))
                  (then
                    (global.set $res_ptr (i32.const 64))
                    (global.set $res_len (i32.const {alen}))
                    (i32.const 1))
                  (else (i32.const 0)))"#,
                ulen = url.len(),
                alen = answer_json().len()
            ),
        )
    };
    let good = mk("https://api.example.com/x");
    let evil = mk("https://evil.example.org/");

    // Deterministic transport: allow-listed origin answers, everything else
    // fails without touching the network.
    let fake = Arc::new(FakeNetworkTransport::new());
    fake.route("api.example.com", FakeOutcome::Status(200, b"{}".to_vec()));

    let host = start_plugin_with_transport(
        "org.example.net",
        &["lyrics:provider", "network:fetch"],
        &["https://api.example.com"],
        &good,
        Some(fake.clone()),
    );
    assert!(host
        .fetch_lyrics("org.example.net", &query())
        .unwrap()
        .is_some());
    assert_eq!(fake.calls(), vec!["https://api.example.com/x".to_string()]);

    let host = PluginHost::new().unwrap();
    host.set_network_transport(fake.clone());
    let manifest = PluginManifest::from_json(&manifest_json_domains(
        "org.example.evil",
        &["lyrics:provider", "network:fetch"],
        &["https://api.example.com"],
    ))
    .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    host.load_bytes("org.example.evil", &wat::parse_str(evil).unwrap())
        .unwrap();
    host.start("org.example.evil").unwrap();
    // Denied at runtime: resolves as a miss, not an error.
    assert!(host
        .fetch_lyrics("org.example.evil", &query())
        .unwrap()
        .is_none());
    // The denied URL never reached the transport.
    assert_eq!(fake.calls(), vec!["https://api.example.com/x".to_string()]);
}

#[test]
fn net_last_status_reports_outcome() {
    use sonora_plugin::{NET_STATUS_TIMEOUT, NET_STATUS_TOO_LARGE};

    let probe = |expected: i32| {
        lyrics_module(
            "(import \"sonora\" \"net_fetch\" (func $net_fetch (param i32 i32 i32 i32) (result i32)))\n  (import \"sonora\" \"net_last_status\" (func $st (result i32)))\n  (data (i32.const 256) \"https://api.example.com/x\")",
            &format!(
                r#"(local $r i32)
                (local.set $r (call $net_fetch (i32.const 256) (i32.const 25) (i32.const 512) (i32.const 1024)))
                (if (result i32) (i32.and
                    (i32.eq (local.get $r) (i32.const -1))
                    (i32.eq (call $st) (i32.const {expected})))
                  (then
                    (global.set $res_ptr (i32.const 64))
                    (global.set $res_len (i32.const {alen}))
                    (i32.const 1))
                  (else (i32.const 0)))"#,
                alen = answer_json().len()
            ),
        )
    };

    // 404 surfaces as the status; the query resolves as a miss at the
    // lyrics layer but the plugin observed the real code.
    let fake = Arc::new(FakeNetworkTransport::new());
    fake.route("api.example.com", FakeOutcome::Status(404, vec![]));
    let host = start_plugin_with_transport(
        "org.example.status",
        &["lyrics:provider", "network:fetch"],
        &["https://api.example.com"],
        &probe(404),
        Some(fake),
    );
    assert!(host
        .fetch_lyrics("org.example.status", &query())
        .unwrap()
        .is_some());

    for (outcome, code) in [
        (FakeOutcome::Timeout, NET_STATUS_TIMEOUT),
        (FakeOutcome::TooLarge, NET_STATUS_TOO_LARGE),
    ] {
        let fake = Arc::new(FakeNetworkTransport::new());
        fake.route("api.example.com", outcome);
        let host = start_plugin_with_transport(
            "org.example.status",
            &["lyrics:provider", "network:fetch"],
            &["https://api.example.com"],
            &probe(code),
            Some(fake),
        );
        assert!(host
            .fetch_lyrics("org.example.status", &query())
            .unwrap()
            .is_some());
    }
}

#[test]
fn library_snapshot_and_tap_are_read_only() {
    // lib_stats probe: hit iff the host snapshot has exactly the expected length.
    let snapshot = serde_json::json!({"tracks": 7});
    let snap_len = snapshot.to_string().len();
    let lib_src = lyrics_module(
        r#"(import "sonora" "lib_stats" (func $lib_stats (param i32 i32) (result i32)))"#,
        &format!(
            r#"(if (result i32) (i32.eq (call $lib_stats (i32.const 512) (i32.const 1024)) (i32.const {snap_len}))
              (then
                (global.set $res_ptr (i32.const 64))
                (global.set $res_len (i32.const {alen}))
                (i32.const 1))
              (else (i32.const 0)))"#,
            alen = answer_json().len()
        ),
    );
    let host = start_plugin(
        "org.example.lib",
        &["lyrics:provider", "library:read"],
        &[],
        &lib_src,
    );
    // Default stub snapshot has a different length: miss.
    assert!(host
        .fetch_lyrics("org.example.lib", &query())
        .unwrap()
        .is_none());
    host.set_library_snapshot(snapshot);
    assert!(host
        .fetch_lyrics("org.example.lib", &query())
        .unwrap()
        .is_some());

    // tap_read probe: hit iff 4 floats arrive with 0.5 as the first sample.
    let tap_src = lyrics_module(
        r#"(import "sonora" "tap_read" (func $tap_read (param i32 i32) (result i32)))"#,
        &format!(
            r#"(local $n i32)
            (local.set $n (call $tap_read (i32.const 512) (i32.const 256)))
            (if (i32.ne (local.get $n) (i32.const 4))
              (then (i32.const 0) (return)))
            (if (i32.ne (i32.load (i32.const 512)) (i32.const 1056964608))
              (then (i32.const 0) (return)))
            (global.set $res_ptr (i32.const 64))
            (global.set $res_len (i32.const {alen}))
            (i32.const 1)"#,
            alen = answer_json().len()
        ),
    );
    let host = PluginHost::new().unwrap();
    let manifest = PluginManifest::from_json(&manifest_json(
        "org.example.tap",
        &["lyrics:provider", "visualizer:tap"],
    ))
    .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    host.load_bytes("org.example.tap", &wat::parse_str(&tap_src).unwrap())
        .unwrap();
    host.start("org.example.tap").unwrap();
    assert!(host
        .fetch_lyrics("org.example.tap", &query())
        .unwrap()
        .is_none());
    host.push_tap_frame(&[0.5, 0.25, 0.125, 0.0625]);
    assert!(host
        .fetch_lyrics("org.example.tap", &query())
        .unwrap()
        .is_some());
}

// ---------------------------------------------------------------------------
// Crash isolation
// ---------------------------------------------------------------------------

#[test]
fn init_trap_marks_crashed_without_harming_host() {
    let src = r#"(module
      (memory (export "memory") 1)
      (func (export "plugin_api_version") (result i32) (i32.const 1))
      (func (export "plugin_init") (result i32) (unreachable)))"#;
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.boom", &["lyrics:provider"]))
            .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    host.load_bytes("org.example.boom", &wat::parse_str(src).unwrap())
        .unwrap();
    assert!(host.start("org.example.boom").is_err());
    assert!(matches!(
        host.state("org.example.boom"),
        Some(PluginState::Crashed { .. })
    ));

    // The host itself is fine: a healthy plugin loads and runs on this host.
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.good2", &["lyrics:provider"]))
            .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    host.load_bytes(
        "org.example.good2",
        &wat::parse_str(lyrics_module("", &hit_body())).unwrap(),
    )
    .unwrap();
    host.start("org.example.good2").unwrap();
    assert!(host
        .fetch_lyrics("org.example.good2", &query())
        .unwrap()
        .is_some());
}

#[test]
fn fetch_trap_is_isolated_to_the_crashing_plugin() {
    let crasher_src = lyrics_module("", "(unreachable)");
    let sibling_src = lyrics_module("", &hit_body());
    let host = PluginHost::new().unwrap();
    for (id, src) in [
        ("org.example.crasher", crasher_src.as_str()),
        ("org.example.sibling", sibling_src.as_str()),
    ] {
        let manifest = PluginManifest::from_json(&manifest_json(id, &["lyrics:provider"])).unwrap();
        host.register(manifest, PathBuf::from("t")).unwrap();
        host.load_bytes(id, &wat::parse_str(src).unwrap()).unwrap();
        host.start(id).unwrap();
    }
    let err = host
        .fetch_lyrics("org.example.crasher", &query())
        .unwrap_err()
        .to_string();
    assert!(err.contains("crashed"), "unexpected error: {err}");
    assert!(matches!(
        host.state("org.example.crasher"),
        Some(PluginState::Crashed { .. })
    ));
    // Sibling is unaffected.
    assert_eq!(
        host.state("org.example.sibling"),
        Some(PluginState::Running)
    );
    assert!(host
        .fetch_lyrics("org.example.sibling", &query())
        .unwrap()
        .is_some());
}

#[test]
fn consecutive_crashes_disable_then_manual_recovery() {
    let crasher = lyrics_module("", "(unreachable)");
    let wasm = wat::parse_str(&crasher).unwrap();
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.flaky", &["lyrics:provider"]))
            .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();

    for _ in 0..3 {
        host.load_bytes("org.example.flaky", &wasm).unwrap();
        host.start("org.example.flaky").unwrap();
        let _ = host.fetch_lyrics("org.example.flaky", &query());
    }
    assert!(matches!(
        host.state("org.example.flaky"),
        Some(PluginState::Disabled { .. })
    ));

    // Manual recovery: reload a healthy module and start; counters reset.
    let healthy = wat::parse_str(lyrics_module("", &hit_body())).unwrap();
    host.load_bytes("org.example.flaky", &healthy).unwrap();
    host.start("org.example.flaky").unwrap();
    assert_eq!(host.state("org.example.flaky"), Some(PluginState::Running));
    assert!(host
        .fetch_lyrics("org.example.flaky", &query())
        .unwrap()
        .is_some());
}

#[test]
fn infinite_loop_is_stopped_by_fuel() {
    let looper = lyrics_module("", "(loop $spin (br $spin)) (unreachable)");
    let host = PluginHost::new().unwrap();
    let manifest =
        PluginManifest::from_json(&manifest_json("org.example.loopy", &["lyrics:provider"]))
            .unwrap();
    host.register(manifest, PathBuf::from("t")).unwrap();
    host.load_bytes("org.example.loopy", &wat::parse_str(&looper).unwrap())
        .unwrap();
    host.start("org.example.loopy").unwrap();
    let err = host
        .fetch_lyrics("org.example.loopy", &query())
        .unwrap_err()
        .to_string();
    assert!(err.contains("crashed"), "unexpected error: {err}");
    assert!(matches!(
        host.state("org.example.loopy"),
        Some(PluginState::Crashed { .. })
    ));
}

// ---------------------------------------------------------------------------
// Checked-in example artifact
// ---------------------------------------------------------------------------

#[test]
fn checked_in_example_plugin_loads_and_answers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/example");
    let manifest = PluginManifest::from_file(&root.join("manifest.json")).unwrap();
    assert_eq!(manifest.id, "org.sonora.example.lyrics");
    let wasm = std::fs::read(root.join("plugin.wasm")).unwrap();

    let host = PluginHost::new().unwrap();
    let id = manifest.id.clone();
    host.register(manifest, root).unwrap();
    host.load_bytes(&id, &wasm).unwrap();
    host.start(&id).unwrap();
    let answer = host.fetch_lyrics(&id, &query()).unwrap().unwrap();
    assert_eq!(answer.format, "lrc");
    assert!(answer.content.contains("Hello from the example plugin"));
}
