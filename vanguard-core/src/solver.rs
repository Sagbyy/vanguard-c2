//! Wire protocol for the external OR-Tools assignment solver. The control host
//! ships a *generic* min-cost-flow graph (nodes + capacitated, costed arcs) and
//! the sidecar returns the flow on each arc. All targeting semantics (which arc
//! means what) stay in the control host — the sidecar is a pure solver.

use serde::{Deserialize, Serialize};

/// Control host → solver: solve a min-cost flow, get per-arc flow back.
pub const SOLVE_ASSIGNMENT: &str = "control.solve.assignment";

/// One directed arc of the flow network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveArc {
    pub from: usize,
    pub to: usize,
    pub capacity: i64,
    /// Unit cost; the solver minimises total cost, so an assignment we want is
    /// encoded as a negative cost (= minus its engagement value).
    pub cost: i64,
}

/// A min-cost-flow problem: push `supply` units from `source` to `sink` at
/// minimum total cost over `arcs`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveRequest {
    pub num_nodes: usize,
    pub source: usize,
    pub sink: usize,
    pub supply: i64,
    pub arcs: Vec<SolveArc>,
}

/// Solver → control host: flow carried by each arc, in the request's arc order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveResponse {
    pub flows: Vec<i64>,
}
