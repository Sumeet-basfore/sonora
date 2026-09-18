//! WASM ABI layer for the LRCLIB provider (wasm32 only).
//!
//! Memory model: three dedicated static regions, no allocator interplay with
//! Rust's own heap.
//!
//! - `ARENA` (1 MiB): host-written request JSON via `alloc` (bump, reset on
//!   every `lyrics_fetch`).
//! - `RESP_BUF` (512 KiB): raw LRCLIB HTTP body via `net_fetch`.
//! - `OUT_BUF` (1 MiB): answer envelope JSON read back by the host.
//!
//! All static access goes through `addr_of_mut!` raw pointers (single-threaded
//! WASM, no aliasing). A Rust panic aborts to a trap; the host isolates it to
//! this plugin.

use crate::client::{answer_json, build_search_url, map_response, LyricsQuery, ResolveOutcome};
use core::{ptr, slice, str};

#[link(wasm_import_module = "sonora")]
extern "C" {
    #[link_name = "log"]
    fn host_log(ptr: *const u8, len: usize) -> i32;
    #[link_name = "net_fetch"]
    fn host_fetch(url_ptr: *const u8, url_len: usize, resp_ptr: *mut u8, resp_cap: usize) -> i32;
    #[link_name = "net_last_status"]
    fn host_last_status() -> i32;
}

fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len());
    }
}

const ARENA_LEN: usize = 1024 * 1024;
const RESP_LEN: usize = 512 * 1024;
const OUT_LEN: usize = 1024 * 1024;

static mut ARENA: [u8; ARENA_LEN] = [0; ARENA_LEN];
static mut ARENA_POS: usize = 0;
static mut RESP_BUF: [u8; RESP_LEN] = [0; RESP_LEN];
static mut OUT_BUF: [u8; OUT_LEN] = [0; OUT_LEN];
static mut RES_LEN: usize = 0;

#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    unsafe {
        let size = size.min(ARENA_LEN);
        let start = ARENA_POS.min(ARENA_LEN - size);
        ARENA_POS = start + size;
        ptr::addr_of_mut!(ARENA).cast::<u8>().add(start)
    }
}

#[no_mangle]
pub extern "C" fn plugin_api_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn plugin_init() -> i32 {
    log("lrclib provider init (org.sonora.lrclib v1.0.0)");
    0
}

#[no_mangle]
pub extern "C" fn plugin_shutdown() -> i32 {
    0
}

fn status_note(status: i32) -> &'static str {
    match status {
        404 => "no lyrics on LRCLIB",
        429 => "LRCLIB rate limit hit",
        -2 => "LRCLIB request timed out",
        -3 => "LRCLIB transport error",
        -4 => "LRCLIB response too large",
        _ => "LRCLIB request failed",
    }
}

#[no_mangle]
pub extern "C" fn lyrics_fetch(ptr: *const u8, len: usize) -> i32 {
    unsafe {
        ARENA_POS = 0;
        RES_LEN = 0;

        // 1. Request JSON (bounded by the host to 1 MiB; re-check locally).
        if len == 0 || len > ARENA_LEN {
            return 0;
        }
        let req_bytes = slice::from_raw_parts(ptr, len);
        let req_text = match str::from_utf8(req_bytes) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let query = match LyricsQuery::from_json(req_text) {
            Ok(q) => q,
            Err(e) => {
                log(&format!("lrclib: bad query ({e})"));
                return 0;
            }
        };

        // 2. Search URL (pure client logic; empty/overlong queries miss).
        let url = match build_search_url(&query) {
            Ok(u) => u,
            Err(e) => {
                log(&format!("lrclib: cannot build search URL ({e})"));
                return 0;
            }
        };

        // 3. Host-mediated fetch. The sandbox performs no I/O itself; the
        // host enforces the https://lrclib.net allow-list, timeout and cap.
        let resp: *mut u8 = ptr::addr_of_mut!(RESP_BUF).cast();
        let n = host_fetch(url.as_ptr(), url.len(), resp, RESP_LEN);
        if n < 0 {
            let status = host_last_status();
            log(&format!("lrclib: {} (status {status})", status_note(status)));
            return 0;
        }
        let body = slice::from_raw_parts(resp as *const u8, n as usize);

        // 4. Map to the canonical answer envelope.
        match map_response(200, body) {
            ResolveOutcome::Hit { format, content } => {
                let answer = answer_json(format, &content);
                if answer.len() > OUT_LEN {
                    log("lrclib: answer too large");
                    return 0;
                }
                ptr::copy_nonoverlapping(
                    answer.as_ptr(),
                    ptr::addr_of_mut!(OUT_BUF).cast(),
                    answer.len(),
                );
                RES_LEN = answer.len();
                1
            }
            ResolveOutcome::Miss => 0,
        }
    }
}

// Result window: absolute linear-memory addresses of OUT_BUF.
#[no_mangle]
pub extern "C" fn lyrics_result_ptr() -> usize {
    unsafe {
        if RES_LEN == 0 {
            return 0;
        }
        ptr::addr_of!(OUT_BUF) as *const u8 as usize
    }
}

#[no_mangle]
pub extern "C" fn lyrics_result_len() -> usize {
    unsafe { RES_LEN }
}
