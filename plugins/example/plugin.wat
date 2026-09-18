;; Example Sonora plugin: minimal WASM lyrics provider.
;;
;; Build: cargo run -p sonora-plugin --example build_example
;;   (or: wat --output plugin.wasm plugin.wat)
;;
;; ABI contract (see plugins/example/README.md):
;; - imports  : (sonora.log) only -- this manifest declares `lyrics:provider`
;; - exports  : memory, alloc, plugin_api_version, plugin_init,
;;              plugin_shutdown, lyrics_fetch + lyrics_result_ptr/len
;; - lyrics_fetch always answers with the static LRC below (status 1 = hit).
(module
  (import "sonora" "log" (func $log (param i32 i32) (result i32)))

  (memory (export "memory") 1)

  ;; Bump allocator for host-written request buffers. Heap starts after the
  ;; static region (log line @0, LRC payload @64).
  (global $heap (mut i32) (i32.const 256))
  (func (export "alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (global.get $heap)
    (local.set $ptr)
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $ptr))

  ;; Static data.
  (data (i32.const 0) "example-plugin init")
  ;; JSON answer envelope (111 bytes): {"format":"lrc","content":"<two-line LRC>"}.
  ;; The host parses this envelope, then parses `content` with its own LRC parser.
  (data (i32.const 64) "{\"format\":\"lrc\",\"content\":\"[00:01.00] Hello from the example plugin\\n[00:05.00] WASM lyrics provider online\\n\"}")

  ;; Result window written by lyrics_fetch.
  (global $res_ptr (mut i32) (i32.const 0))
  (global $res_len (mut i32) (i32.const 0))

  (func (export "plugin_api_version") (result i32)
    (i32.const 1))

  (func (export "plugin_init") (result i32)
    ;; "example-plugin init" is 19 bytes at offset 0.
    (call $log (i32.const 0) (i32.const 19))
    (drop)
    (i32.const 0))

  (func (export "plugin_shutdown") (result i32)
    (i32.const 0))

  ;; Always-hit demo provider: answers with the static 111-byte JSON envelope.
  ;; Real providers would parse the JSON query at ($ptr, $len) first.
  (func (export "lyrics_fetch") (param $ptr i32) (param $len i32) (result i32)
    (global.set $res_ptr (i32.const 64))
    (global.set $res_len (i32.const 111))
    (i32.const 1))

  (func (export "lyrics_result_ptr") (result i32)
    (global.get $res_ptr))

  (func (export "lyrics_result_len") (result i32)
    (global.get $res_len)))
