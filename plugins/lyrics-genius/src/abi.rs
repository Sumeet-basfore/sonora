//! WASM ABI layer for the Genius provider (wasm32 only).
//!
//! Memory model: three dedicated static regions, no allocator interplay with
//! Rust's own heap.
//!
//! - `ARENA` (1 MiB): host-written request JSON via `alloc` (bump, reset on
//!   every `lyrics_fetch`).
//! - `RESP_BUF` (2 MiB): raw HTTP bodies via `net_fetch` (reused across the
//!   search + song-page round-trips; sized for real ~600 KiB song pages).
//! - `OUT_BUF` (1 MiB): answer envelope JSON read back by the host.
//!
//! All static access goes through `addr_of_mut!` raw pointers (single-threaded
//! WASM, no aliasing). A Rust panic aborts to a trap; the host isolates it to
//! this plugin.

use crate::client::{
    answer_json, build_search_url, extract_lyrics, parse_search_response, LyricsQuery,
};
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
const RESP_LEN: usize = 2 * 1024 * 1024;
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
    log("genius provider init (org.sonora.lyrics_genius v1.0.0)");
    0
}

#[no_mangle]
pub extern "C" fn plugin_shutdown() -> i32 {
    0
}

fn status_note(step: &str, status: i32) -> &'static str {
    let _ = step;
    match status {
        404 => "no match on Genius",
        429 => "Genius rate limit hit",
        403 => "Genius refused the request",
        -2 => "Genius request timed out",
        -3 => "Genius transport error",
        -4 => "Genius response too large",
        _ => "Genius request failed",
    }
}

/// One host-mediated GET. Returns the body on HTTP 200, `None` otherwise
/// (status already recorded by the host for logging).
fn fetch_url(url: &str) -> Option<usize> {
    unsafe {
        let resp: *mut u8 = ptr::addr_of_mut!(RESP_BUF).cast();
        let n = host_fetch(url.as_ptr(), url.len(), resp, RESP_LEN);
        if n < 0 {
            return None;
        }
        Some(n as usize)
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
                log(&format!("genius: bad query ({e})"));
                return 0;
            }
        };

        // 2. Search URL (pure client logic; empty/overlong queries miss).
        let search_url = match build_search_url(&query) {
            Ok(u) => u,
            Err(e) => {
                log(&format!("genius: cannot build search URL ({e})"));
                return 0;
            }
        };

        // 3. Round-trip 1: search API -> song URL. Every fetch goes through
        // the host, which enforces the genius.com allow-list, timeout, cap,
        // and no-redirect policy. No sockets, files, or processes here.
        let Some(n) = fetch_url(&search_url) else {
            let status = host_last_status();
            log(&format!("genius: search {} (status {status})", status_note("search", status)));
            return 0;
        };
        let search_body = slice::from_raw_parts(ptr::addr_of_mut!(RESP_BUF).cast::<u8>().cast_const(), n);
        let song_url = match parse_search_response(search_body) {
            Some(u) => u,
            None => {
                log("genius: no song hit in search response");
                return 0;
            }
        };

        // 4. Round-trip 2: song page -> lyrics HTML.
        let Some(m) = fetch_url(&song_url) else {
            let status = host_last_status();
            log(&format!("genius: song page {} (status {status})", status_note("page", status)));
            return 0;
        };
        let page_body = slice::from_raw_parts(ptr::addr_of_mut!(RESP_BUF).cast::<u8>().cast_const(), m);
        let lyrics = match extract_lyrics(page_body) {
            Some(t) => t,
            None => {
                log("genius: song page has no usable lyrics containers");
                return 0;
            }
        };

        // 5. Canonical answer envelope with attribution.
        let answer = answer_json(&lyrics, &song_url);
        if answer.len() > OUT_LEN {
            log("genius: answer too large");
            return 0;
        }
        ptr::copy_nonoverlapping(answer.as_ptr(), ptr::addr_of_mut!(OUT_BUF).cast(), answer.len());
        RES_LEN = answer.len();
        1
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
