# Particle Simulation

**Live demo:** https://galmungral.github.io/wasm-particle-sim/

## Rhetorical Design

### Purpose

For an audience familiar with particle simulations or physics engines, we want
to communicate two things. First, that the choice of data structure is not a
matter of theory — it is what determines whether a simulation can be interactive
at all. Naively checking every pair of particles for collision is $`O(N^2)`$; a
uniform spatial grid reduces this to $`O(N)`$.

Second, that Rust compiled to WebAssembly is substantially faster than an
equivalent JavaScript implementation for this kind of workload — fast enough to
change what particle counts are feasible in a browser.

### Strategy

**Interactive parameter control.** The demo exposes particle count as a
real-time slider and displays the frame rate continuously. The purpose is to let
the viewer push $`N`$ high enough to observe the difference between $`O(N^2)`$
and $`O(N)`$ directly. A toggle switches between the two collision detection code
paths. The brute-force implementation remains in `simulation.rs` for direct
comparison.

## Technical Implementation

### 1. Uniform spatial grid

At each timestep, particles are binned into a 3D grid where each cell has side
length $`2 \times r_{\max}`$. To find collision candidates for a given particle,
only the $`3 \times 3 \times 3 = 27`$ neighboring cells need to be examined.
With particles distributed uniformly, each cell holds $`O(1)`$ particles on
average, so the total work across all $`N`$ particles is $`O(N)`$ rather than
$`O(N^2)`$. The grid is rebuilt each frame as particles move.

### 2. Architecture

The simulation, rendering, and UI are all written in Rust and compiled to WASM
via `wasm-bindgen`. This is functional, but WebGL calls and DOM manipulation in
Rust are verbose — the language is well-suited to the simulation logic, but adds
unnecessary friction everywhere else.

The [wasm-fvm-cfd](https://github.com/GalMunGral/wasm-fvm-cfd) project
establishes a cleaner boundary: the numerical solver runs in WASM, while
everything that touches the browser — rendering, controls, animation loop — is
written in TypeScript. The two layers communicate through a thin exported API.