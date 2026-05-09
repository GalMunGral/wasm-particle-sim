# particle-effects

**Live demo:** https://galmungral.github.io/particle-effects/

## Rhetorical Design

### Purpose

For an audience familiar with particle simulations or physics engines, we want
to communicate two things. First, that the choice of an appropriate data
structure is not a matter of theory — it is what determines whether a simulation
can be interactive at all. Naively checking every pair of particles for
collision is $`O(N^2)`$; a uniform spatial grid brings this down to $`O(N)`$.

Second, that Rust compiled to WebAssembly is not marginally faster than
JavaScript for this kind of workload — it is substantially faster, enough to
change what particle counts are feasible in a browser.

### Strategy

**Interactive parameter control.** The demo lets you vary the number of
particles in real time and watch the frame rate respond. The point is not just
that the simulation runs — it is that you can push $`N`$ high enough to make the
difference between $`O(N^2)`$ and $`O(N)`$ visible. The commented-out brute-force
loop in `simulation.rs` is there for direct comparison.

## Technical Implementation

### 1. Uniform spatial grid

At each timestep, particles are binned into a 3D grid where each cell has side
length $`2 \times r_{\max}`$. To find collision candidates for a given particle,
only the $`3 \times 3 \times 3 = 27`$ neighboring cells are checked. With
particles distributed uniformly, each cell holds $`O(1)`$ particles on average,
so the total work across all particles is $`O(N)`$ rather than $`O(N^2)`$.

The grid is rebuilt every frame because particles move.

### 2. Architecture

The simulation, rendering, and UI are all written in Rust and compiled to WASM
via `wasm-bindgen`. This works, but the WebGL calls and DOM manipulation in Rust
are verbose and awkward — the language and its borrow checker are well-suited to
the simulation logic, but add friction everywhere else.

The [wasm-fvm-cfd](https://github.com/GalMunGral/wasm-fvm-cfd) project arrived
at a cleaner split: the solver runs in WASM, and everything that touches the
browser — rendering, controls, animation loop — is written in TypeScript. The
two communicate through a thin exported API. That boundary is the right one.