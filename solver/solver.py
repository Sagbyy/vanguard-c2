#!/usr/bin/env python3
"""OR-Tools assignment sidecar.

Subscribes to the `control.solve.assignment` NATS subject and answers each
request with a min-cost-flow solution. The control host (vanguard-control) ships
a generic flow graph; we hand it straight to OR-Tools' SimpleMinCostFlow and
return the flow carried by every arc, in request order. All targeting semantics
live on the Rust side — this process is a pure solver.

Run a NATS broker first (`docker run -p 4222:4222 nats:latest`), then:
    pip install -r requirements.txt
    python solver.py            # or NATS_URL=nats://host:4222 python solver.py
"""

import asyncio
import json
import os

import nats
from ortools.graph.python import min_cost_flow

SUBJECT = "control.solve.assignment"
DEFAULT_NATS_URL = "nats://127.0.0.1:4222"


def solve(req: dict) -> dict:
    """Run SimpleMinCostFlow on the request graph, return per-arc flow."""
    mcf = min_cost_flow.SimpleMinCostFlow()

    # Arc order is the contract: flows[i] must describe req["arcs"][i].
    arc_ids = []
    for arc in req["arcs"]:
        arc_ids.append(
            mcf.add_arc_with_capacity_and_unit_cost(
                arc["from"], arc["to"], arc["capacity"], arc["cost"]
            )
        )

    # Push `supply` units source -> sink. A 0-cost bypass arc (added by the
    # caller) absorbs whatever can't be matched, so the problem stays feasible.
    supply = req["supply"]
    mcf.set_node_supply(req["source"], supply)
    mcf.set_node_supply(req["sink"], -supply)

    status = mcf.solve()
    if status != mcf.OPTIMAL:
        # Report no flow rather than guess; the host falls back locally.
        return {"flows": [0] * len(arc_ids)}

    return {"flows": [mcf.flow(a) for a in arc_ids]}


async def main() -> None:
    url = os.environ.get("NATS_URL", DEFAULT_NATS_URL)
    nc = await nats.connect(url)
    print(f"or-tools solver online via {url}, listening on {SUBJECT}")

    async def handler(msg):
        try:
            req = json.loads(msg.data)
            resp = solve(req)
        except Exception as error:  # never drop a reply: the host is waiting
            print(f"solve error: {error}")
            resp = {"flows": []}
        await msg.respond(json.dumps(resp).encode())

    await nc.subscribe(SUBJECT, cb=handler)
    # Block forever; the subscription callback does the work.
    await asyncio.Event().wait()


if __name__ == "__main__":
    asyncio.run(main())
