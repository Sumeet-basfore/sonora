//! Integration tests for the Spectrum+ visualizer (`plugins/spectrum-plus`).
//!
//! The plugin only uses the pre-existing ABI (`tap_read` import,
//! `visualizer_info` export): no host or ABI changes were needed. Tests drive
//! [`WasmPluginInstance`] directly with scripted tap frames.

use sonora_plugin::{
    Capability, PluginManifest, PluginState, RealNetworkTransport, WasmPluginInstance,
};
use std::path::PathBuf;
use std::sync::Arc;

const PLUGIN_ID: &str = "org.sonora.spectrum_plus";

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/spectrum-plus")
}

fn real_manifest() -> PluginManifest {
    PluginManifest::from_file(&plugin_dir().join("manifest.json")).expect("real manifest")
}

fn test_engine() -> wasmtime::Engine {
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    wasmtime::Engine::new(&config).expect("engine")
}

fn started_instance() -> WasmPluginInstance {
    let engine = test_engine();
    let manifest = real_manifest();
    let wasm = std::fs::read(plugin_dir().join("plugin.wasm")).expect("built plugin.wasm");
    let transport: Arc<dyn sonora_plugin::NetworkTransport> = Arc::new(RealNetworkTransport::new());
    let mut inst = WasmPluginInstance::load(&engine, manifest, &wasm, transport).expect("load");
    inst.start().expect("start");
    inst
}

fn info_json(inst: &mut WasmPluginInstance) -> serde_json::Value {
    let text = inst.visualizer_info().expect("visualizer_info");
    serde_json::from_str(&text).expect("descriptor JSON")
}

#[test]
fn real_manifest_declares_tap_only() {
    let m = real_manifest();
    assert_eq!(m.id, PLUGIN_ID);
    assert_eq!(m.api_version, 1);
    assert_eq!(m.entry, "plugin.wasm");
    assert!(m.capabilities.contains(Capability::VisualizerTap));
    assert_eq!(m.capabilities.len(), 1);
    assert!(m.allowed_domains.is_empty());
}

#[test]
fn real_plugin_dir_discovers() {
    let plugins_root = plugin_dir().parent().expect("plugins root").to_path_buf();
    let host = sonora_plugin::PluginHost::new().unwrap();
    let found = host.discover(&plugins_root);
    assert!(
        found.contains(&PLUGIN_ID.to_string()),
        "discovered: {found:?}"
    );
    assert_eq!(host.state(PLUGIN_ID), Some(PluginState::Validated));
}

#[test]
fn descriptor_shape_without_tap_is_silence() {
    let mut inst = started_instance();
    // No tap pushed: silence frame, descriptor still answers.
    let info = info_json(&mut inst);
    assert_eq!(info["name"], "Spectrum+");
    assert_eq!(info["modes"], serde_json::json!(["bars", "wave", "mirror"]));
    assert_eq!(info["preferred_fps"], 60);
    assert_eq!(info["uses_audio_tap"], true);
    assert_eq!(info["frame"]["bins"], 0);
    for mode in ["bars", "wave", "mirror"] {
        let arr = info["frame"][mode].as_array().unwrap();
        assert_eq!(arr.len(), 64);
        assert!(arr.iter().all(|v| v.as_f64().unwrap_or(-1.0) == 0.0));
    }
    let logs = inst.take_logs();
    assert!(
        logs.iter().any(|l| l.contains("spectrum+")),
        "logs: {logs:?}"
    );
}

#[test]
fn spike_maps_to_expected_bar_and_mirror_is_symmetric() {
    let mut inst = started_instance();
    // 256-bin spike at bin 130 -> bar 32 (bins [128,132)).
    let mut frame = vec![0.0f32; 256];
    frame[130] = 1.0;
    inst.push_tap_frame(frame);
    let info = info_json(&mut inst);
    assert_eq!(info["frame"]["bins"], 256);
    let bars = info["frame"]["bars"].as_array().unwrap();
    assert_eq!(bars.len(), 64);
    assert!((bars[32].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert!(bars
        .iter()
        .enumerate()
        .all(|(i, v)| i == 32 || v.as_f64().unwrap() == 0.0));
    let mirror = info["frame"]["mirror"].as_array().unwrap();
    for i in 0..64 {
        let a = mirror[i].as_f64().unwrap();
        let b = mirror[63 - i].as_f64().unwrap();
        assert!((a - b).abs() < 1e-9, "mirror not symmetric at {i}");
    }
    // Mirror of a bar-32 spike lands on bars 31/32.
    assert!(mirror[31].as_f64().unwrap() > 0.0);
    assert!(mirror[32].as_f64().unwrap() > 0.0);
    assert_eq!(info["frame"]["wave"].as_array().unwrap().len(), 64);
}

#[test]
fn release_smoothing_decays_without_retrigger() {
    let mut inst = started_instance();
    inst.push_tap_frame(vec![1.0f32; 64]);
    let hot: f64 = info_json(&mut inst)["frame"]["bars"][0].as_f64().unwrap();
    assert!((hot - 1.0).abs() < 1e-6);
    inst.push_tap_frame(vec![0.0f32; 64]);
    let decayed: f64 = info_json(&mut inst)["frame"]["bars"][0].as_f64().unwrap();
    assert!(
        decayed > 0.0 && decayed < 1.0,
        "expected decay, got {decayed}"
    );
}

#[test]
fn missing_tap_capability_rejects_load() {
    let engine = test_engine();
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(plugin_dir().join("manifest.json")).unwrap())
            .unwrap();
    value["capabilities"] = serde_json::json!(["storage:cache"]);
    value["allowed_domains"] = serde_json::json!([]);
    let manifest = PluginManifest::from_json(&value.to_string()).unwrap();
    let wasm = std::fs::read(plugin_dir().join("plugin.wasm")).unwrap();
    let transport: Arc<dyn sonora_plugin::NetworkTransport> = Arc::new(RealNetworkTransport::new());
    let err = match WasmPluginInstance::load(&engine, manifest, &wasm, transport) {
        Ok(_) => panic!("load without visualizer:tap should fail"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("visualizer:tap"), "unexpected error: {err}");
}

#[test]
fn print_example_frame() {
    // Documentation source for the README ASCII sketch below. Run with
    // --nocapture to regenerate the example output.
    let mut inst = started_instance();
    let frame: Vec<f32> = (0..256)
        .map(|i| (i as f32 / 256.0 * std::f32::consts::PI * 6.0).sin().abs())
        .collect();
    inst.push_tap_frame(frame);
    let info = info_json(&mut inst);
    let bars = info["frame"]["bars"].as_array().unwrap();
    let heights: Vec<usize> = bars
        .iter()
        .map(|v| (v.as_f64().unwrap() * 8.0).round() as usize)
        .collect();
    println!("example bars (64): {heights:?}");
    for row in (1..=8).rev() {
        let line: String = heights
            .iter()
            .map(|&h| if h >= row { '#' } else { '.' })
            .collect();
        println!("{line}");
    }
}
