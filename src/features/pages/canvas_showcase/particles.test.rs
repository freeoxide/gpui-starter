use gpui_kit::{Entity, TestAppContext, VisualTestContext, point, px};

use super::{FIELD_H, FIELD_W, Particle, ParticleFieldDemo, seed_particles, tick};

fn particle(pos: (f32, f32), vel: (f32, f32), diameter: f32) -> Particle {
    Particle {
        pos: point(px(pos.0), px(pos.1)),
        vel: point(px(vel.0), px(vel.1)),
        diameter: px(diameter),
    }
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected ~{expected}, got {actual}"
    );
}

#[test]
fn seed_particles_is_deterministic() {
    assert_eq!(seed_particles(), seed_particles());
}

#[test]
fn seeded_particles_start_strictly_inside_the_field() {
    for particle in seed_particles() {
        let r = particle.diameter.as_f32() / 2.;
        assert!(particle.pos.x.as_f32() >= r);
        assert!(particle.pos.x.as_f32() <= FIELD_W - r);
        assert!(particle.pos.y.as_f32() >= r);
        assert!(particle.pos.y.as_f32() <= FIELD_H - r);
    }
}

#[test]
fn tick_moves_each_particle_by_exactly_its_velocity() {
    let mut particles = vec![particle((100., 100.), (2., 3.), 8.)];
    tick(&mut particles);
    assert_f32_eq(particles[0].pos.x.as_f32(), 102.);
    assert_f32_eq(particles[0].pos.y.as_f32(), 103.);
    assert_f32_eq(particles[0].vel.x.as_f32(), 2.);
    assert_f32_eq(particles[0].vel.y.as_f32(), 3.);
}

#[test]
fn tick_reflects_and_clamps_at_the_low_walls() {
    let mut particles = vec![particle((3., 3.), (-2., -1.), 8.)];
    tick(&mut particles);
    assert_f32_eq(particles[0].pos.x.as_f32(), 4.);
    assert_f32_eq(particles[0].pos.y.as_f32(), 4.);
    assert_f32_eq(particles[0].vel.x.as_f32(), 2.);
    assert_f32_eq(particles[0].vel.y.as_f32(), 1.);
}

#[test]
fn tick_reflects_and_clamps_at_the_high_walls() {
    let mut particles = vec![particle((798., 498.), (2., 1.), 8.)];
    tick(&mut particles);
    assert_f32_eq(particles[0].pos.x.as_f32(), 796.);
    assert_f32_eq(particles[0].pos.y.as_f32(), 496.);
    assert_f32_eq(particles[0].vel.x.as_f32(), -2.);
    assert_f32_eq(particles[0].vel.y.as_f32(), -1.);
}

#[test]
fn no_particle_escapes_across_six_hundred_ticks() {
    let mut particles = seed_particles();
    for _ in 0..600 {
        tick(&mut particles);
    }
    for particle in &particles {
        let r = particle.diameter.as_f32() / 2.;
        assert!(
            particle.pos.x.as_f32() >= r,
            "x escaped: {}",
            particle.pos.x
        );
        assert!(
            particle.pos.x.as_f32() <= FIELD_W - r,
            "x escaped: {}",
            particle.pos.x
        );
        assert!(
            particle.pos.y.as_f32() >= r,
            "y escaped: {}",
            particle.pos.y
        );
        assert!(
            particle.pos.y.as_f32() <= FIELD_H - r,
            "y escaped: {}",
            particle.pos.y
        );
    }
}

#[gpui_kit::test]
fn paused_render_draws_without_advancing_positions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| ParticleFieldDemo::new(cx));
    let demo: Entity<ParticleFieldDemo> = handle.root(cx).expect("demo root entity");
    demo.update(cx, |field, _| field.playing = false);
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    let before = demo.update(&mut cx, |field, _| field.particles.clone());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let after = demo.update(&mut cx, |field, _| field.particles.clone());
    assert_eq!(before, after);
}
