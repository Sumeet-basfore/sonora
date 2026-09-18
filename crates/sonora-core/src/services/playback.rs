//! Playback service: audio output control plane.
//!
//! Owns the [`AudioPlayer`] handle (the control side; decoding and the
//! real-time thread stay inside `sonora-audio`). Every mutation publishes the
//! corresponding playback event; reads are side-effect free.

use crate::bus::EventBus;
use crate::event::SonoraEvent;
use sonora_audio::{AudioPlayer, PlayerStateSnapshot};
use sonora_common::{PlaybackState, Result};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlaybackService {
    player: Arc<AudioPlayer>,
    bus: EventBus,
}

impl PlaybackService {
    pub fn new(player: Arc<AudioPlayer>, bus: EventBus) -> Self {
        Self { player, bus }
    }

    pub fn play_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.player.play_file(path)?;
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.player.pause()?;
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Paused));
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        self.player.resume()?;
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.player.stop()?;
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Stopped));
        Ok(())
    }

    pub fn seek(&self, position_ms: u64) -> Result<()> {
        self.player.seek(position_ms)
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let clamped = volume.clamp(0.0, 2.0);
        self.player.set_volume(clamped)?;
        self.bus.publish(SonoraEvent::VolumeChanged {
            volume: clamped,
            muted: clamped <= 0.0,
        });
        Ok(())
    }

    pub fn announce_track(
        &self,
        track_id: sonora_common::TrackId,
        title: String,
        artist: Option<String>,
        duration_ms: u64,
    ) {
        self.bus.publish(SonoraEvent::TrackChanged {
            track_id,
            title,
            artist,
            duration_ms,
        });
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Playing));
    }

    pub fn announce_stopped(&self) {
        self.bus
            .publish(SonoraEvent::PlaybackStateChanged(PlaybackState::Stopped));
    }

    pub fn state(&self) -> PlaybackState {
        self.player.state()
    }

    pub fn volume(&self) -> f32 {
        self.player.volume()
    }

    pub fn position_ms(&self) -> u64 {
        self.player.position_ms()
    }

    pub fn is_finished(&self) -> bool {
        self.player.is_finished()
    }

    pub fn snapshot(&self) -> PlayerStateSnapshot {
        self.player.snapshot()
    }

    pub fn visualizer_data(&self) -> Vec<f32> {
        self.player.visualizer_data()
    }

    pub fn player(&self) -> &AudioPlayer {
        &self.player
    }
}
