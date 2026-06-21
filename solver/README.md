# OR-Tools assignment sidecar

A pure min-cost-flow solver exposed on the NATS bus. `vanguard-control` builds the
interceptor→threat assignment as a flow graph and ships it here every tick; this
process feeds it to OR-Tools' `SimpleMinCostFlow` and returns the flow on each arc.

Keeping the solver here (rather than as a Rust FFI binding) is what lets us use the
*real* OR-Tools library. The control host degrades gracefully: if this sidecar is
not running, it falls back to its in-process Hungarian assignment.

## Run

```bash
docker run -p 4222:4222 nats:latest        # broker (if not already up)
pip install -r requirements.txt
python solver.py                            # NATS_URL overrides the broker URL
```

Then start the rest of the stack (`cargo run -p vanguard-control`, etc.).

## What the host encodes in the graph

- **Range** → an interceptor↔threat arc only exists when the threat is inside the
  platform's reach (no arc instead of a giant penalty).
- **Ammo / reload** → the number of launch slots (source→shooter capacity) per
  platform is bounded by its remaining interceptors.
- **Saturation** → the top-tier threats get a threat→sink capacity of 2, so two
  interceptors can be committed to the most dangerous targets.
