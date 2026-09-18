//! WASM ABI layer for the Spectrum+ visualizer (wasm32 only).
//!
//! Memory model: two dedicated static regions, no allocator interplay with
//! Rust's own heap.
//!
//! - `TAP_BUF` (1 KiB): latest tap frame via `tap_read` (256 f32 LE max).
//! - `OUT_BUF` (8 KiB): descriptor + computed frame JSON read by the host.
//!
//! `PREV` holds the previous smoothed bars for release decay. All static
//! access goes through `addr_of_mut!` raw pointers (single-threaded WASM, no
//! aliasing). A Rust panic aborts to a trap; the host isolates it to this
//! plugin.

use crate::client::{
    compute_bars, compute_mirror, compute_wave, descriptor_json, smooth, FrameData, MAX_TAP,
    OUT_BINS,
};
use core::ptr;

#[link(wasm_import_module = "sonora")]
extern "C" {
    #[link_name = "log"]
    fn host_log(ptr: *const u8, len: usize) -> i32;
    #[link_name = "tap_read"]
    fn host_tap(obuf: i32, ocap: i32) -> i32;
}

fn log(msg: &str) {
    unsafe {
        host_log(msg.as_ptr(), msg.len());
    }
}

const TAP_LEN: usize = MAX_TAP * 4;
const OUT_LEN: usize = 8 * 1024;

static mut TAP_BUF: [u8; TAP_LEN] = [0; TAP_LEN];
static mut OUT_BUF: [u8; OUT_LEN] = [0; OUT_LEN];
static mut PREV: [f32; OUT_BINS] = [0.0; OUT_BINS];
static mut RES_LEN: usize = 0;

#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    // Required by the host for any query-style export; this plugin takes no
    // input, so hand out a scratch window inside TAP_BUF.
    let _ = size.min(TAP_LEN);
    ptr::addr_of_mut!(TAP_BUF).cast::<u8>()
}

#[no_mangle]
pub extern "C" fn plugin_api_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn plugin_init() -> i32 {
    log("spectrum+ visualizer init (org.sonora.spectrum_plus v1.0.0)");
    0
}

#[no_mangle]
pub extern "C" fn plugin_shutdown() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn visualizer_info() -> i32 {
    unsafe {
        RES_LEN = 0;
        // Pull the latest read-only tap frame; an empty tap still yields a
        // valid (zeroed) descriptor so clients can render silence.
        let tap: *mut u8 = ptr::addr_of_mut!(TAP_BUF).cast();
        let n = host_tap(tap as i32, TAP_LEN as i32);
        let mut frame = [0.0f32; MAX_TAP];
        let bins = if n > 0 {
            let count = (n as usize).min(MAX_TAP);
            for i in 0..count {
                let mut word = [0u8; 4];
                ptr::copy_nonoverlapping(tap.add(i * 4), word.as_mut_ptr(), 4);
                frame[i] = f32::from_le_bytes(word);
            }
            count
        } else {
            0
        };

        let prev: [f32; OUT_BINS] = ptr::addr_of!(PREV).read();
        let bars = smooth(&prev, &compute_bars(&frame[..bins]));
        ptr::addr_of_mut!(PREV).write(bars);
        let wave = compute_wave(&frame[..bins]);
        let mirror = compute_mirror(&bars);
        let data = FrameData {
            bins,
            bars: bars.to_vec(),
            wave: wave.to_vec(),
            mirror: mirror.to_vec(),
        };
        let answer = descriptor_json(&data);
        if answer.len() > OUT_LEN {
            log("spectrum+: answer too large");
            return 0;
        }
        ptr::copy_nonoverlapping(answer.as_ptr(), ptr::addr_of_mut!(OUT_BUF).cast(), answer.len());
        RES_LEN = answer.len();
        1
    }
}

#[no_mangle]
pub extern "C" fn visualizer_result_ptr() -> usize {
    unsafe {
        if RES_LEN == 0 {
            return 0;
        }
        ptr::addr_of!(OUT_BUF) as *const u8 as usize
    }
}

#[no_mangle]
pub extern "C" fn visualizer_result_len() -> usize {
    unsafe { RES_LEN }
}
