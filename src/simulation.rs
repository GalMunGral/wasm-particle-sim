use std::cell::RefCell;

use nalgebra::Vector3;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Vector3<f32> {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h6 = h * 6.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Vector3::new(r + m, g + m, b + m)
}

use crate::utils::{
    math::{clamp, rand_range},
    web::random_f32,
};

const BOX_SIZE: f32 = 4.0;
const G_GRAVITY: Vector3<f32> = Vector3::new(0.0, 0.0, -9.80665);
const E_SPHERE: f32 = 0.9;
const E_WALL: f32 = 0.6;
const C_AIR: f32 = 0.05;

pub struct Clock {
    prev_time: f32,
    pub last_dt: f32,
}

impl Clock {
    pub fn new() -> Clock {
        Clock { prev_time: 0.0, last_dt: 0.0 }
    }
    pub fn reset(&mut self) {
        self.prev_time = 0.0;
        self.last_dt = 0.0;
    }
    pub fn advance(&mut self, time: f32) -> f32 {
        let dt = if self.prev_time > 0.0 { time - self.prev_time } else { 0.0 };
        self.last_dt = dt;
        self.prev_time = time;
        dt
    }
}

#[derive(Clone, Copy)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub radius: f32,
    pub mass: f32,
    pub color: Vector3<f32>,
}

impl Particle {
    pub fn random(min_radius: f32, max_radius: f32) -> Particle {
        let radius = rand_range(min_radius, max_radius);
        Particle {
            position: random_f32()
                * 0.5
                * BOX_SIZE
                * Vector3::new(random_f32() - 0.5, random_f32() - 0.5, random_f32() - 0.5)
                    .normalize(),
            velocity: random_f32()
                * 5.0
                * BOX_SIZE
                * Vector3::new(random_f32() - 0.5, random_f32() - 0.5, random_f32() - 0.5)
                    .normalize(),
            mass: radius.powi(3),
            radius,
            color: hsl_to_rgb(random_f32(), 0.75, 0.65),
        }
    }
}

fn collide(p1: &RefCell<Particle>, p2: &RefCell<Particle>) {
    if p1.as_ptr() == p2.as_ptr() {
        return;
    }
    let mut p1 = p1.borrow_mut();
    let mut p2 = p2.borrow_mut();
    let mut d = p2.position - p1.position;
    let dist = d.norm();
    if dist > p1.radius + p2.radius {
        return;
    }
    d.normalize_mut();
    let s1 = p1.velocity.dot(&d);
    let s2 = p2.velocity.dot(&d);
    let s = s1 - s2;
    if s <= 0.0 {
        return;
    }
    let w1 = p2.mass / (p1.mass + p2.mass);
    let w2 = p1.mass / (p1.mass + p2.mass);
    p1.velocity -= w1 * (1.0 + E_SPHERE) * s * d;
    p2.velocity += w2 * (1.0 + E_SPHERE) * s * d;
}

pub struct Simulation {
    pub particles: Vec<RefCell<Particle>>,
    pub particle_count: u32,
    pub use_grid: bool,
    min_radius: f32,
    max_radius: f32,
    clock: Clock,
}

impl Simulation {
    pub fn new() -> Simulation {
        Simulation {
            particle_count: 0,
            use_grid: true,
            min_radius: 0.0,
            max_radius: 0.0,
            clock: Clock::new(),
            particles: Vec::new(),
        }
    }
    pub fn fps(&self) -> f32 {
        1.0 / self.clock.last_dt
    }

    pub fn repeat(&mut self) {
        self.reset(self.particle_count);
    }

    pub fn reset(&mut self, particle_count: u32) {
        self.particle_count = particle_count;
        self.min_radius = 0.15 * BOX_SIZE / f32::sqrt(self.particle_count as f32);
        self.max_radius = 4.0 * self.min_radius;
        self.particles.clear();
        for _ in 0..particle_count {
            self.particles.push(RefCell::new(Particle::random(
                self.min_radius,
                self.max_radius,
            )));
        }
        self.clock.reset();
    }

    pub fn advance(&mut self, timestamp: f32) {
        let dt = self.clock.advance(timestamp);

        for p in self.particles.iter_mut() {
            let p = &mut *p.borrow_mut();
            p.position += p.velocity * dt;
            // HACK
            for i in 0..3 {
                p.position[i] = clamp(p.position[i], -0.5 * BOX_SIZE, 0.5 * BOX_SIZE);
            }
        }

        for p in self.particles.iter_mut() {
            let p = &mut *p.borrow_mut();
            p.velocity += G_GRAVITY * dt;
            p.velocity -= C_AIR * (p.radius.powi(2) / p.mass) * p.velocity * dt;
            for i in 0..3 {
                if 0.5 * BOX_SIZE - p.position[i] <= p.radius && p.velocity[i] > 0.0
                    || p.position[i] + 0.5 * BOX_SIZE <= p.radius && p.velocity[i] < 0.0
                {
                    p.velocity[i] = -E_WALL * p.velocity[i];
                }
            }
        }

        if !self.use_grid {
            for p1 in self.particles.iter() {
                for p2 in self.particles.iter() {
                    collide(p1, p2);
                }
            }
            return;
        }

        let grid_size = 2.0 * self.max_radius;
        let dim = f32::floor(BOX_SIZE / grid_size) as usize + 1;
        let mut grid = vec![Vec::<&RefCell<Particle>>::new(); dim * dim * dim];

        let index = |x: f32| f32::floor((x + 0.5 * BOX_SIZE) / grid_size) as usize;

        macro_rules! ind {
            ($i:expr, $j:expr, $k:expr) => {
                (($i * dim) + $j) * dim + $k
            };
        }

        macro_rules! ind_f {
            ($x:expr, $y:expr, $z:expr) => {
                ((index($x) * dim) + index($y)) * dim + index($z)
            };
        }

        for p in self.particles.iter() {
            let pos = p.borrow().position;
            grid[ind_f!(pos.x, pos.y, pos.z)].push(p);
        }

        for p1 in self.particles.iter() {
            let (i, j, k) = {
                // need to drop this borrow before collide is called bellow
                let p = p1.borrow();
                (
                    index(p.position.x),
                    index(p.position.y),
                    index(p.position.z),
                )
            };
            for gi in [i - 1, i, i + 1] {
                for gj in [j - 1, j, j + 1] {
                    for gk in [k - 1, k, k + 1] {
                        if gi < dim && gj < dim && gk < dim {
                            for p2 in grid[ind!(gi, gj, gk)].iter() {
                                collide(p1, p2);
                            }
                        }
                    }
                }
            }
        }
    }
}
