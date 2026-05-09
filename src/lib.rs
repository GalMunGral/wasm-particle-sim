mod simulation;
mod utils;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

use nalgebra::{Matrix4, Point3, Vector3};
use simulation::Simulation;
use std::{cell::RefCell, convert::TryInto, f32::consts::PI, rc::Rc};
use utils::{
    geometry::make_sphere,
    set_panic_hook,
    web::{clear_interval, document, request_animation_frame, set_interval, window},
    webgl::{load_shaders, setup_geometry, Geometry},
};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{
    HtmlCanvasElement, HtmlInputElement, WebGl2RenderingContext, WebGlProgram,
    WebGlVertexArrayObject,
};

#[derive(Debug)]
struct Camera {
    eye: Point3<f32>,
    forward: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
}

impl Camera {
    fn new(x: f32, y: f32, z: f32) -> Camera {
        let mut camera = Camera {
            eye: Point3::origin(),
            forward: Vector3::zeros(),
            right: Vector3::zeros(),
            up: Vector3::zeros(),
        };
        camera.eye = Point3::new(x, y, z);
        camera.forward = (Point3::origin() - camera.eye).normalize();
        let up = Vector3::new(0.0, 0.0, 1.0);
        camera.right = camera.forward.cross(&up).normalize();
        camera.up = camera.right.cross(&camera.forward);
        camera
    }
}

const VERTEX_SHADER_URL: &str = "sphere_vertex.glsl";
const FRAGMENT_SHADER_URL: &str = "sphere_fragment.glsl";
const BOX_SIZE: f32 = 4.0;
const NEAR_PLANE: f32 = 1.0;
const FAR_PLANE: f32 = 10.0 * BOX_SIZE;
const FOCAL_LENGTH: f32 = 200.0;
const RESET_DELAY_MS: i32 = 5000;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    set_panic_hook();

    let camera = Camera::new(1.5 * BOX_SIZE, -1.5 * BOX_SIZE, 0.75 * BOX_SIZE);

    let proj_mat = Rc::new(RefCell::new(Matrix4::<f32>::zeros()));

    let gl = document()
        .query_selector("canvas")?
        .ok_or("element canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?
        .get_context("webgl2")?
        .ok_or("can't get webgl2 context")?
        .dyn_into::<WebGl2RenderingContext>()?;

    gl.enable(WebGl2RenderingContext::DEPTH_TEST);

    let program = load_shaders(&gl, VERTEX_SHADER_URL, FRAGMENT_SHADER_URL).await?;
    let geometry = make_sphere();
    let vao = setup_geometry(&gl, &program, &geometry)?;

    let simulation = Rc::new(RefCell::new(Simulation::new()));
    simulation.borrow_mut().reset(50);

    setup_event_listeners(&gl, &proj_mat, &simulation)?;

    let raf_cb = Rc::new(RefCell::new(None as Option<Closure<dyn FnMut(f32)>>));
    let raf_cb_ = Rc::clone(&raf_cb);
    *raf_cb.borrow_mut() = Some(Closure::new(move |timestamp| {
        draw(
            &gl,
            &program,
            &vao,
            &geometry,
            &camera,
            &proj_mat,
            &simulation,
            timestamp / 1000.0,
        );
        request_animation_frame(raf_cb_.borrow().as_ref().expect("RAF callback is defined"))
            .expect("RAF failed");
    }));
    request_animation_frame(raf_cb.borrow().as_ref().expect("RAF callback is defined"))
        .expect("RAF failed");

    Ok(())
}

fn draw(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    vao: &WebGlVertexArrayObject,
    geometry: &Geometry,
    camera: &Camera,
    proj_mat: &Rc<RefCell<Matrix4<f32>>>,
    simulation: &Rc<RefCell<Simulation>>,
    timestamp: f32,
) {
    document()
        .query_selector("#fps")
        .unwrap()
        .unwrap()
        .set_text_content(Some(&{
    let fps = simulation.borrow().fps();
    if fps.is_finite() { format!("{:.0} fps", fps) } else { String::new() }
}));

    simulation.borrow_mut().advance(timestamp);

    gl.clear_color(0.05, 0.05, 0.08, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    gl.use_program(Some(program));
    gl.bind_vertex_array(Some(vao));

    gl.uniform3fv_with_f32_array(
        gl.get_uniform_location(&program, "eye").as_ref(),
        (camera.eye - Point3::origin()).as_ref(),
    );

    let target = camera.eye + FOCAL_LENGTH * camera.forward;
    let view_mat = Matrix4::look_at_rh(&camera.eye, &target, &camera.up);
    gl.uniform_matrix4fv_with_f32_array(
        gl.get_uniform_location(&program, "v").as_ref(),
        false,
        view_mat.as_slice(),
    );

    gl.uniform_matrix4fv_with_f32_array(
        gl.get_uniform_location(&program, "p").as_ref(),
        false,
        proj_mat.borrow().as_slice(),
    );

    for p in simulation.borrow().particles.iter() {
        let p = p.borrow();
        let model_mat = Matrix4::new_translation(&p.position) * Matrix4::new_scaling(p.radius);
        gl.uniform_matrix4fv_with_f32_array(
            gl.get_uniform_location(&program, "m").as_ref(),
            false,
            model_mat.as_slice(),
        );

        let color_attr_loc: u32 = gl
            .get_attrib_location(program, "color")
            .try_into()
            .expect("color not defined in shader");
        gl.vertex_attrib3fv_with_f32_array(color_attr_loc, p.color.as_slice());

        gl.draw_elements_with_i32(
            WebGl2RenderingContext::TRIANGLES,
            geometry
                .triangles
                .len()
                .try_into()
                .expect("number of indices should fit into i32"),
            WebGl2RenderingContext::UNSIGNED_INT,
            0,
        );
    }
}

fn reset_aspect_ratio(
    gl: &WebGl2RenderingContext,
    proj_mat: &Rc<RefCell<Matrix4<f32>>>,
) -> Result<(), JsValue> {
    let canvas = document()
        .query_selector("canvas")?
        .ok_or("element canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;

    canvas.set_width(
        canvas
            .client_width()
            .try_into()
            .expect("canvas width shouldn't be negative"),
    );
    canvas.set_height(
        canvas
            .client_height()
            .try_into()
            .expect("canvas height shouldn't be negative"),
    );

    *proj_mat.borrow_mut() = Matrix4::new_perspective(
        canvas.width() as f32 / canvas.height() as f32,
        PI / 3.0,
        NEAR_PLANE,
        FAR_PLANE,
    );

    gl.viewport(
        0,
        0,
        canvas
            .width()
            .try_into()
            .expect("canvas width is non-negative"),
        canvas
            .height()
            .try_into()
            .expect("canvas height is non-negative"),
    );
    Ok(())
}

fn setup_event_listeners(
    gl: &WebGl2RenderingContext,
    proj_mat: &Rc<RefCell<Matrix4<f32>>>,
    simulation: &Rc<RefCell<Simulation>>,
) -> Result<(), JsValue> {
    let gl_1 = gl.clone();
    let proj_mat_1 = Rc::clone(proj_mat);
    reset_aspect_ratio(&gl, &proj_mat_1)?;

    let onresize = Closure::<dyn FnMut()>::new(move || {
        reset_aspect_ratio(&gl_1, &proj_mat_1).expect("failed to reset aspect ratio");
    });
    window().add_event_listener_with_callback("resize", onresize.as_ref().unchecked_ref())?;
    onresize.forget();

    let count = document().query_selector("#count")?.ok_or("#count")?;
    let control = document()
        .query_selector("#particle-count")?
        .ok_or("#particle-count")?
        .dyn_into::<HtmlInputElement>()?;
    count.set_text_content(Some(&format!("number of particles: {}", control.value())));

    let mut timer_handle = 0;
    let simulation_1 = Rc::clone(simulation);
    let control_1 = control.clone();
    let reset_cb = Closure::<dyn FnMut()>::new(move || {
        count.set_text_content(Some(&format!("number of particles: {}", control_1.value())));
        reset_timer(&simulation_1, &mut timer_handle).expect("failed to reset timer");
        simulation_1
            .borrow_mut()
            .reset(control_1.value().parse::<u32>().unwrap_or(50));
    });
    control.add_event_listener_with_callback("input", reset_cb.as_ref().unchecked_ref())?;
    reset_cb.forget();

    let toggle = document()
        .query_selector("#use-grid")?
        .ok_or("#use-grid")?
        .dyn_into::<HtmlInputElement>()?;
    let simulation_2 = Rc::clone(simulation);
    let toggle_1 = toggle.clone();
    let toggle_cb = Closure::<dyn FnMut()>::new(move || {
        simulation_2.borrow_mut().use_grid = toggle_1.checked();
    });
    toggle.add_event_listener_with_callback("change", toggle_cb.as_ref().unchecked_ref())?;
    toggle_cb.forget();

    Ok(())
}

fn reset_timer(simulation: &Rc<RefCell<Simulation>>, handle: &mut i32) -> Result<(), JsValue> {
    clear_interval(*handle);
    let simulation_1 = Rc::clone(simulation);
    let reset_cb = Closure::<dyn FnMut()>::new(move || simulation_1.borrow_mut().repeat());
    *handle = set_interval(&reset_cb, RESET_DELAY_MS)?;
    reset_cb.forget();
    Ok(())
}
