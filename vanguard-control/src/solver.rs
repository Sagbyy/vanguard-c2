//! Thin client to the external OR-Tools sidecar (see `solver/solver.py`). One
//! NATS request-reply per assignment. Any failure (sidecar down, timeout, bad
//! reply) returns `None` so the caller can fall back to its local solver.

use std::time::Duration;

use vanguard_core::{SOLVE_ASSIGNMENT, SolveRequest, SolveResponse};

/// Budget for one solve round-trip. The tick is ~1-2 s; the solve itself is
/// sub-millisecond, so this only guards against a missing/stuck sidecar.
const SOLVE_TIMEOUT: Duration = Duration::from_millis(250);

pub struct Solver {
    client: async_nats::Client,
}

impl Solver {
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }

    /// Send the flow graph to the sidecar and return the per-arc flow, or `None`
    /// if the sidecar did not answer in time / with a well-formed response.
    pub async fn solve(&self, req: &SolveRequest) -> Option<SolveResponse> {
        let payload = serde_json::to_vec(req).ok()?;
        let reply = tokio::time::timeout(
            SOLVE_TIMEOUT,
            self.client.request(SOLVE_ASSIGNMENT, payload.into()),
        )
        .await
        .ok()?
        .ok()?;
        let resp: SolveResponse = serde_json::from_slice(&reply.payload).ok()?;
        (resp.flows.len() == req.arcs.len()).then_some(resp)
    }
}
