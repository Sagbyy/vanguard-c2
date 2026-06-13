use std::collections::HashMap;

use uuid::Uuid;

use vanguard_core::{DetectedThreat, ThreatTrack};

const MATCH_DISTANCE: f64 = 50.0;

pub fn update_track(
    tracks: &mut HashMap<Uuid, ThreatTrack>,
    threat: DetectedThreat,
    source_platform: Uuid,
) -> ThreatTrack {
    for track in tracks.values_mut() {
        if track.position.distance(&threat.position) < MATCH_DISTANCE {
            track.position.x = (track.position.x + threat.position.x) / 2.0;

            track.position.y = (track.position.y + threat.position.y) / 2.0;

            track.velocity = threat.speed.clone();

            track.confidence = track.confidence.max(threat.confidence);

            track.last_update = threat.detected_at;

            if !track.source_platforms.contains(&source_platform) {
                track.source_platforms.push(source_platform);
            }

            return track.clone();
        }
    }

    let track = ThreatTrack {
        track_id: Uuid::new_v4(),
        position: threat.position,
        velocity: threat.speed,
        confidence: threat.confidence,
        threat_level: threat.threat_level,
        last_update: threat.detected_at,
        source_platforms: vec![source_platform],
    };

    tracks.insert(track.track_id, track.clone());

    track
}

pub fn cleanup_tracks(tracks: &mut HashMap<Uuid, ThreatTrack>, now: f64) {
    tracks.retain(|_, track| now - track.last_update < 10.0);
}
