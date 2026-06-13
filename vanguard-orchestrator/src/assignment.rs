use vanguard_core::{Assignment, InterceptorState, ThreatTrack};

use crate::state::InterceptorInfo;

pub fn compute_assignments(
    tracks: &[ThreatTrack],
    interceptors: &[InterceptorInfo],
) -> Vec<Assignment> {
    let mut assignments = Vec::new();

    let mut available_tracks = tracks.to_vec();

    available_tracks.sort_by(|a, b| b.threat_level.cmp(&a.threat_level));

    for interceptor in interceptors {
        if !matches!(interceptor.interceptor.state, InterceptorState::Idle) {
            continue;
        }

        if interceptor.interceptor.assigned_track.is_some() {
            continue;
        }

        let Some(best_idx) = available_tracks
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                interceptor
                    .interceptor
                    .position
                    .distance(&a.position)
                    .partial_cmp(&interceptor.interceptor.position.distance(&b.position))
                    .unwrap()
            })
            .map(|(idx, _)| idx)
        else {
            break;
        };

        let track = available_tracks.remove(best_idx);

        assignments.push(Assignment {
            platform_id: interceptor.platform_id,
            interceptor_id: interceptor.interceptor.id,
            track_id: track.track_id,
        });
    }

    assignments
}
