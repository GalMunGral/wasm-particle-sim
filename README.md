# Particle Simulation

**Live demo:** https://galmungral.github.io/wasm-particle-sim/

## Rhetorical Design

### Purpose

Most interactive programs process a fixed, small amount of data per user action,
so algorithmic complexity and runtime rarely matter — the input is bounded
regardless of $`N`$. A real-time particle simulation is one of the few contexts
where $`N`$ is something you can actually turn up, and both factors become
visible at once.

**Data structure:** the naive $`O(N^2)`$ collision check and the $`O(N)`$ spatial
grid produce observably different frame rates once $`N`$ is large enough.

**Runtime:** at that scale, JavaScript's runtime type guards and GC pauses
become a bottleneck too — which is why this project uses WASM for the simulation
and WebGL for rendering rather than staying in JavaScript.

### Strategy

**Interactive parameter control.** The demo exposes particle count as a
real-time slider and displays the frame rate continuously. The purpose is to let
the viewer push $`N`$ high enough to observe the difference between $`O(N^2)`$
and $`O(N)`$ directly. A toggle switches between the two collision detection code
paths.

**A stepping stone.** Everything here — simulation, rendering, and UI — is
written in Rust and compiled to WASM. This works, but WebGL calls and DOM
manipulation in Rust are verbose and awkward. The lesson informed
[wasm-fvm-cfd](https://github.com/GalMunGral/wasm-fvm-cfd), which draws a
cleaner boundary: the solver in WASM, everything that touches the browser in
TypeScript.

## Technical Challenges

### 1. Collision detection

The naive approach checks every pair of particles, giving $`O(N^2)`$ work per
frame. Two approaches are possible:

| | Brute force | Uniform spatial grid |
|---|---|---|
| **Complexity** | $`O(N^2)`$ | $`O(N)`$ |
| **Memory** | None | $`O(N)`$ grid rebuilt each frame |
| **Implementation** | Two nested loops | Bin particles by position, check 27 neighbors |
| **Correctness** | Exact | Exact — no false negatives, cell size guarantees all candidates are in adjacent cells |

The grid cell size is set to $`2 \times r_{\max}`$, so any two overlapping
particles are guaranteed to be in the same or adjacent cells. Each particle
checks at most $`3^3 = 27`$ cells; with uniform distribution each cell holds
$`O(1)`$ particles on average, giving $`O(N)`$ total.