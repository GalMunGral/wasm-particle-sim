# Particle Simulation

**Live demo:** https://galmungral.github.io/wasm-particle-sim/

## Rhetorical Design

### Purpose

This project demonstrates two points. Most interactive programs are I/O bound —
they spend their time waiting on user input or network responses, so algorithmic
complexity rarely surfaces. A real-time simulation is compute bound, which makes
it one of the few contexts where data structure choice is immediately visible.
The naive $`O(N^2)`$ collision check and the $`O(N)`$ spatial grid produce
observably different frame rates at realistic particle counts.

Execution model also matters at scale. JavaScript's JIT compiler must guard
against type changes at runtime and can deoptimize; WASM's static types are
resolved at compile time, producing tight machine code without runtime checks.
On top of that, GC pauses in JavaScript are unpredictable in a way that is
particularly disruptive for a simulation that needs to hit 60fps consistently.

### Strategy

**Interactive parameter control.** The demo exposes particle count as a
real-time slider and displays the frame rate continuously. The purpose is to let
the viewer push $`N`$ high enough to observe the difference between $`O(N^2)`$
and $`O(N)`$ directly. A toggle switches between the two collision detection code
paths. 
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

### 2. Architecture

The simulation, rendering, and UI are all written in Rust and compiled to WASM
via `wasm-bindgen`. This is functional, but WebGL calls and DOM manipulation in
Rust are verbose — the language is well-suited to the simulation logic, but adds
unnecessary friction everywhere else.

The [wasm-fvm-cfd](https://github.com/GalMunGral/wasm-fvm-cfd) project
establishes a cleaner boundary: the numerical solver runs in WASM, while
everything that touches the browser — rendering, controls, animation loop — is
written in TypeScript. The two layers communicate through a thin exported API.