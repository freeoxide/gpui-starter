use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{Canvas, *};

use super::geometry::square_at;

const FIELD_W: f32 = 800.;
const FIELD_H: f32 = 500.;
const PARTICLE_COUNT: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    pos: Point<Pixels>,
    vel: Point<Pixels>,
    diameter: Pixels,
}

struct XorShift32(u32);

impl XorShift32 {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

fn seed_particles() -> Vec<Particle> {
    let mut rng = XorShift32(0x9E37_79B9);
    (0..PARTICLE_COUNT)
        .map(|_| {
            let x = 20. + (rng.next() % 760) as f32;
            let y = 20. + (rng.next() % 460) as f32;
            let speed_x = 0.5 + (rng.next() % 30) as f32 * 0.05;
            let sign_x = if rng.next() & 1 == 0 { 1. } else { -1. };
            let speed_y = 0.5 + (rng.next() % 30) as f32 * 0.05;
            let sign_y = if rng.next() & 1 == 0 { 1. } else { -1. };
            let diameter = px(6. + (rng.next() % 9) as f32);
            Particle {
                pos: point(px(x), px(y)),
                vel: point(px(speed_x * sign_x), px(speed_y * sign_y)),
                diameter,
            }
        })
        .collect()
}

fn tick(particles: &mut [Particle]) {
    for particle in particles {
        particle.pos += particle.vel;
        let r = particle.diameter.as_f32() / 2.;
        if particle.pos.x.as_f32() < r {
            particle.pos.x = px(r);
            particle.vel.x = px(particle.vel.x.as_f32().abs());
        }
        if particle.pos.x.as_f32() > FIELD_W - r {
            particle.pos.x = px(FIELD_W - r);
            particle.vel.x = px(-particle.vel.x.as_f32().abs());
        }
        if particle.pos.y.as_f32() < r {
            particle.pos.y = px(r);
            particle.vel.y = px(particle.vel.y.as_f32().abs());
        }
        if particle.pos.y.as_f32() > FIELD_H - r {
            particle.pos.y = px(FIELD_H - r);
            particle.vel.y = px(-particle.vel.y.as_f32().abs());
        }
    }
}

pub(super) struct ParticleFieldDemo {
    state: Entity<CanvasState>,
    particles: Vec<Particle>,
    ids: Vec<ShapeId>,
    playing: bool,
    _subscription: Subscription,
}

impl ParticleFieldDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| CanvasState::new());
        let particles = seed_particles();
        let ids = state.update(cx, |s, cx| {
            s.set_builtin_tool(BuiltinTool::Pan, cx);
            s.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            );
            let layer = s.add_layer(cx);
            let colors = {
                let theme = cx.theme();
                [theme.primary, theme.accent, theme.secondary]
            };
            let particles = &particles;
            s.edit_document(cx, |doc| {
                particles
                    .iter()
                    .enumerate()
                    .map(|(i, particle)| {
                        doc.add_shape(
                            layer,
                            CanvasShape::ellipse(
                                square_at(particle.pos, particle.diameter),
                                ShapeStyle::filled(colors[i % 3].opacity(0.85)),
                            ),
                        )
                        .unwrap_or_default()
                    })
                    .collect()
            })
        });
        let _subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            particles,
            ids,
            playing: true,
            _subscription,
        }
    }

    fn write_positions(&mut self, cx: &mut Context<Self>) {
        let positions: Vec<_> = self
            .particles
            .iter()
            .map(|particle| square_at(particle.pos, particle.diameter))
            .collect();
        let ids = self.ids.clone();
        self.state.update(cx, |s, cx| {
            s.edit_document(cx, |doc| {
                for (id, bounds) in ids.iter().zip(&positions) {
                    doc.update_shape(*id, |shape| {
                        if let CanvasShape::Ellipse { bounds: b, .. } = shape {
                            *b = *bounds;
                        }
                    });
                }
            })
        });
    }

    fn toggle_playing(&mut self, cx: &mut Context<Self>) {
        self.playing = !self.playing;
        cx.notify();
    }

    fn reset_field(&mut self, cx: &mut Context<Self>) {
        self.particles = seed_particles();
        self.write_positions(cx);
    }
}

impl Render for ParticleFieldDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.playing && window.is_window_active() && !cx.reduce_motion() {
            tick(&mut self.particles);
            self.write_positions(cx);
            window.request_animation_frame();
        }
        let (toggle_label, toggle_icon) = if self.playing {
            ("showcase_pause", IconName::Pause)
        } else {
            ("showcase_play", IconName::Play)
        };
        super::render_demo_shell(
            "particles",
            crate::i18n::localize(super::CanvasDemo::Particles.title_key()),
            crate::i18n::localize(super::CanvasDemo::Particles.desc_key()),
            vec![
                Button::new("particles-toggle")
                    .ghost()
                    .small()
                    .icon(toggle_icon)
                    .label(crate::i18n::localize(toggle_label))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_playing(cx)))
                    .into_any_element(),
                Button::new("particles-reset")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset"))
                    .on_click(cx.listener(|this, _, _, cx| this.reset_field(cx)))
                    .into_any_element(),
            ],
            Canvas::new("particles-canvas", self.state.clone()),
            cx,
        )
    }
}

#[cfg(test)]
#[path = "particles.test.rs"]
mod particles_test;
