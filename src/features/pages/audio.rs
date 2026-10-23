use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, IndexPath,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle},
    h_flex,
    select::{Select, SelectEvent, SelectState},
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
    v_flex,
};
use gpui_kit::{actions, prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::audio::{AudioEngine, PEAK_COLUMNS, PlaybackStatus, WaveformPeaks};
use crate::ui::waveform::{Waveform, WaveformMode, WaveformState};

actions!(audio_page, [TogglePlayback]);

#[derive(Clone)]
struct DragSeek(EntityId);

impl Render for DragSeek {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

fn format_time(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    format!("{}:{:02}", total_secs / 60, total_secs % 60)
}

fn fraction_at(x: Pixels, bounds: Bounds<Pixels>) -> f32 {
    ((x - bounds.left()) / bounds.size.width).clamp(0., 1.)
}

fn paint_peaks(
    bounds: Bounds<Pixels>,
    peaks: &WaveformPeaks,
    progress: f32,
    window: &mut Window,
    cx: &App,
) {
    let scale = window.scale_factor();
    let width_px = ((bounds.size.width.as_f32() * scale).round().max(1.)) as usize;
    let played = cx.theme().primary.opacity(0.9);
    let unplayed = cx.theme().muted_foreground.opacity(0.45);
    for x in 0..width_px {
        let fraction = x as f32 / width_px as f32;
        let amplitude = peaks
            .peak((fraction * PEAK_COLUMNS as f32) as usize)
            .map(|column| column.max.abs().max(column.min.abs()))
            .unwrap_or(0.)
            .min(1.);
        let span = ((amplitude * bounds.size.height.as_f32() * 0.5) * scale).round() / scale;
        let span = span.max(1. / scale);
        let center = (bounds.center().y.as_f32() * scale).round() / scale;
        let left = bounds.left().as_f32() + x as f32 / scale;
        window.paint_quad(fill(
            Bounds {
                origin: point(px(left), px(center - span / 2.)),
                size: size(px(1. / scale), px(span)),
            },
            if fraction <= progress {
                played
            } else {
                unplayed
            },
        ));
    }
    let playhead_x =
        bounds.left().as_f32() + (progress * bounds.size.width.as_f32() * scale).round() / scale;
    window.paint_quad(fill(
        Bounds {
            origin: point(px(playhead_x), bounds.origin.y),
            size: size(px(1. / scale), bounds.size.height),
        },
        cx.theme().foreground,
    ));
}

fn status_key(status: PlaybackStatus) -> &'static str {
    match status {
        PlaybackStatus::Loading => "audio_status_loading",
        PlaybackStatus::Stopped => "audio_status_stopped",
        PlaybackStatus::Playing => "audio_status_playing",
        PlaybackStatus::Paused => "audio_status_paused",
    }
}

/// Transport controls (play/pause, stop) stay unusable until a load lands.
fn transport_idle(status: Option<PlaybackStatus>) -> bool {
    matches!(status, None | Some(PlaybackStatus::Loading))
}

fn error_key(error: &str) -> &'static str {
    const OUTPUT_PREFIXES: [&str; 4] = [
        "output failed",
        "no output device",
        "device unsupported",
        "playback unavailable",
    ];
    if OUTPUT_PREFIXES
        .iter()
        .any(|prefix| error.starts_with(prefix))
    {
        "audio_error_output"
    } else {
        "audio_error_decode"
    }
}

pub struct AudioPage {
    engine: Entity<AudioEngine>,
    waveform: Entity<WaveformState>,
    volume_slider: Entity<SliderState>,
    device_select: Entity<SelectState<Vec<String>>>,
    device_names: Vec<String>,
    spectrum: bool,
    focus_handle: FocusHandle,
    overview_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    _subscriptions: Vec<Subscription>,
}

impl AudioPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.bind_keys([KeyBinding::new("space", TogglePlayback, Some("AudioPage"))]);
        let engine = cx.new(AudioEngine::new);
        let waveform = cx.new(|_| WaveformState::new());
        let volume = engine.read(cx).volume();
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(volume)
        });
        engine.update(cx, |engine, cx| {
            engine.attach_meter(&waveform, cx);
            engine.refresh_devices(cx);
        });
        let devices = engine.read(cx).devices();
        let selected_index = devices
            .iter()
            .position(|device| device.is_default)
            .map(IndexPath::new);
        let device_names: Vec<String> = devices.iter().map(|device| device.name.clone()).collect();
        let device_select =
            cx.new(|cx| SelectState::new(device_names.clone(), selected_index, window, cx));

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&engine, |_, _, cx| cx.notify()));
        subscriptions.push(cx.observe(&waveform, |_, _, cx| cx.notify()));
        subscriptions.push(cx.observe_in(&engine, window, |this, _, window, cx| {
            this.reconcile_devices(window, cx);
        }));
        subscriptions.push(
            cx.subscribe(
                &volume_slider,
                |this, _, event: &SliderEvent, cx| match event {
                    SliderEvent::Change(value) | SliderEvent::Release(value) => {
                        this.engine
                            .update(cx, |engine, cx| engine.set_volume(value.start(), cx));
                    }
                },
            ),
        );
        subscriptions.push(cx.subscribe(
            &device_select,
            |this, _, event: &SelectEvent<Vec<String>>, cx| {
                let SelectEvent::Confirm(value) = event;
                if let Some(name) = value.clone() {
                    this.engine
                        .update(cx, |engine, cx| engine.select_device(&name, cx));
                }
            },
        ));

        Self {
            engine,
            waveform,
            volume_slider,
            device_select,
            device_names,
            spectrum: false,
            focus_handle: cx.focus_handle().tab_stop(true),
            overview_bounds: Rc::new(RefCell::new(None)),
            _subscriptions: subscriptions,
        }
    }

    fn reconcile_devices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let names = self
            .engine
            .read(cx)
            .devices()
            .iter()
            .map(|device| device.name.clone())
            .collect::<Vec<_>>();
        if names == self.device_names {
            return;
        }
        self.device_select
            .update(cx, |state, cx| state.set_items(names.clone(), window, cx));
        self.device_names = names;
    }

    fn set_spectrum(&mut self, checked: bool, cx: &mut Context<Self>) {
        self.spectrum = checked;
        let mode = if checked {
            WaveformMode::Spectrum
        } else {
            WaveformMode::Oscilloscope
        };
        self.waveform
            .update(cx, |state, cx| state.set_mode(mode, cx));
        cx.notify();
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_load_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let loading = matches!(self.engine.read(cx).status(), Some(PlaybackStatus::Loading));
        let open_engine = self.engine.clone();
        h_flex()
            .gap_2()
            .flex_wrap()
            .items_center()
            .child(
                Button::new("audio-open")
                    .outline()
                    .icon(IconName::FolderOpen)
                    .label(crate::i18n::localize("audio_open_file"))
                    .disabled(loading)
                    .on_click(move |_, _, cx| {
                        if let Some(path) = crate::desktop_actions::pick_file(cx) {
                            open_engine.update(cx, |engine, cx| engine.load_path(path, cx));
                        }
                    }),
            )
            .child(
                Button::new("audio-demo-sweep")
                    .ghost()
                    .label(crate::i18n::localize("audio_demo_sweep"))
                    .disabled(loading)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.load_demo("sine-sweep.wav", 10, crate::audio::sweep_wav, cx)
                    })),
            )
            .child(
                Button::new("audio-demo-chords")
                    .ghost()
                    .label(crate::i18n::localize("audio_demo_chords"))
                    .disabled(loading)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.load_demo("chords.wav", 8, crate::audio::chords_wav, cx)
                    })),
            )
    }

    /// Generates `seconds` of demo audio off-thread, then loads it; the
    /// generators run hundreds of thousands of sin/powf evaluations.
    fn load_demo(
        &mut self,
        name: &'static str,
        seconds: u32,
        generate: fn(u32) -> Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        let bytes = cx.background_spawn(async move { generate(seconds) });
        cx.spawn(async move |this, cx| {
            let bytes = bytes.await;
            this.update(cx, |page, cx| {
                page.engine
                    .update(cx, |engine, cx| engine.load_bytes(name, bytes, cx));
            })
            .ok();
        })
        .detach();
    }

    fn render_meta_line(&self, cx: &Context<Self>) -> impl IntoElement {
        let meta = self.engine.read(cx).meta();
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(match meta {
                Some(meta) => crate::i18n::localize_with_args(
                    "audio_meta_format",
                    &[
                        ("channels", &meta.channels.to_string()),
                        ("rate", &meta.sample_rate.to_string()),
                        ("duration", &format_time(meta.duration)),
                    ],
                ),
                None => String::new(),
            })
    }

    fn render_error_alert(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let error = self.engine.read(cx).last_error()?;
        Some(Alert::warning(
            "audio-error-alert",
            crate::i18n::localize_with_args(error_key(&error), &[("error", &error)]),
        ))
    }

    fn render_empty(&self) -> impl IntoElement {
        Empty::new().header(
            EmptyHeader::new()
                .title(EmptyTitle::new().child(crate::i18n::localize("audio_empty_title")))
                .description(
                    EmptyDescription::new().child(crate::i18n::localize("audio_empty_description")),
                ),
        )
    }

    fn render_unsupported(&self) -> impl IntoElement {
        Empty::new().header(
            EmptyHeader::new()
                .title(EmptyTitle::new().child(crate::i18n::localize("audio_unsupported_title")))
                .description(
                    EmptyDescription::new()
                        .child(crate::i18n::localize("audio_unsupported_description")),
                ),
        )
    }

    fn render_overview(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let engine = self.engine.clone();
        let position = engine.read(cx).position();
        let bounds_cell = self.overview_bounds.clone();
        let canvas_bounds_cell = self.overview_bounds.clone();
        let paint_engine = self.engine.clone();
        let toggle_engine = self.engine.clone();
        let seek_engine = self.engine.clone();
        let drag_engine = self.engine.clone();
        let drag_bounds_cell = self.overview_bounds.clone();
        let expected_id = self.engine.entity_id();

        div()
            .id("audio-overview")
            .relative()
            .h_24()
            .w_full()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .track_focus(&self.focus_handle)
            .a11y(Role::Slider, crate::i18n::localize("audio_seek_label"))
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(1.)
            .aria_value(format_time(position))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                toggle_engine.update(cx, |engine, cx| engine.toggle_play(cx));
            })
            .child(
                canvas(
                    move |bounds, _, _| *canvas_bounds_cell.borrow_mut() = Some(bounds),
                    move |bounds, _, window, cx| {
                        let progress = paint_engine.read(cx).progress();
                        if let Some(peaks) = paint_engine.read(cx).peaks() {
                            paint_peaks(bounds, &peaks, progress, window, cx);
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .id("audio-scrub")
                    .absolute()
                    .size_full()
                    .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                        if let Some(bounds) = *bounds_cell.borrow() {
                            let fraction = fraction_at(event.position.x, bounds);
                            seek_engine.update(cx, |engine, cx| engine.seek_fraction(fraction, cx));
                        }
                    })
                    .on_drag(DragSeek(self.engine.entity_id()), |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &drag_engine,
                        move |engine, event: &DragMoveEvent<DragSeek>, _, cx| {
                            let DragSeek(id) = event.drag(cx);
                            if *id == expected_id
                                && let Some(bounds) = *drag_bounds_cell.borrow()
                            {
                                let fraction = fraction_at(event.event.position.x, bounds);
                                engine.seek_fraction(fraction, cx);
                            }
                        },
                    )),
            )
    }

    fn render_transport(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.engine.read(cx).status();
        let playing = matches!(status, Some(PlaybackStatus::Playing));
        let idle = transport_idle(status);
        let position = self.engine.read(cx).position();
        let duration = self.engine.read(cx).duration().unwrap_or_default();
        let toggle_label =
            crate::i18n::localize(if playing { "audio_pause" } else { "audio_play" });
        let stop_label = crate::i18n::localize("audio_stop");

        h_flex()
            .gap_2()
            .items_center()
            .flex_wrap()
            .child(
                Button::new("audio-toggle")
                    .ghost()
                    .icon(if playing {
                        IconName::Pause
                    } else {
                        IconName::Play
                    })
                    .accessibility_label(toggle_label.clone())
                    .tooltip(toggle_label)
                    .disabled(idle)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.engine.update(cx, |engine, cx| engine.toggle_play(cx));
                    })),
            )
            .child(
                Button::new("audio-stop")
                    .ghost()
                    .icon(IconName::Square)
                    .accessibility_label(stop_label.clone())
                    .tooltip(stop_label)
                    .disabled(idle)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.engine.update(cx, |engine, cx| engine.stop(cx));
                    })),
            )
            .child(
                div()
                    .id("audio-time")
                    .a11y(
                        Role::Paragraph,
                        format!("{} / {}", format_time(position), format_time(duration)),
                    )
                    .a11y_live(accesskit::Live::Polite)
                    .text_sm()
                    .child(format!(
                        "{} / {}",
                        format_time(position),
                        format_time(duration)
                    )),
            )
            .children(status.map(|status| {
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize(status_key(status)))
            }))
            .child(Icon::new(IconName::Settings2))
            .child(
                Slider::new(&self.volume_slider)
                    .horizontal()
                    .w_32()
                    .accessibility_label(crate::i18n::localize("audio_volume")),
            )
            .child(
                Button::new("audio-device-refresh")
                    .ghost()
                    .icon(IconName::RotateCw)
                    .accessibility_label(crate::i18n::localize("audio_device_refresh"))
                    .tooltip(crate::i18n::localize("audio_device_refresh"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.engine
                            .update(cx, |engine, cx| engine.refresh_devices(cx));
                    })),
            )
            .child(
                Select::new(&self.device_select)
                    .placeholder(crate::i18n::localize("audio_device"))
                    .accessibility_label(crate::i18n::localize("audio_device"))
                    .w_48()
                    .disabled(cfg!(target_family = "wasm")),
            )
            .child(
                Switch::new("audio-spectrum")
                    .checked(self.spectrum)
                    .label(crate::i18n::localize("audio_spectrum"))
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_spectrum(*checked, cx);
                    })),
            )
    }

    fn render_live(&self, cx: &Context<Self>) -> impl IntoElement {
        let attached = self.waveform.read(cx).has_source();
        v_flex()
            .gap_2()
            .child(
                div()
                    .id("audio-live-title")
                    .a11y(Role::Heading, crate::i18n::localize("audio_live_title"))
                    .aria_level(2)
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child(crate::i18n::localize("audio_live_title")),
            )
            .child(if attached {
                div()
                    .h_40()
                    .w_full()
                    .border_1()
                    .rounded(cx.theme().radius)
                    .overflow_hidden()
                    .child(
                        Waveform::new("audio-live", self.waveform.clone())
                            .accent(cx.theme().primary),
                    )
                    .into_any_element()
            } else {
                Empty::new()
                    .header(
                        EmptyHeader::new().title(
                            EmptyTitle::new().child(crate::i18n::localize("audio_empty_title")),
                        ),
                    )
                    .into_any_element()
            })
    }

    fn playback_supported(&self, cx: &Context<Self>) -> bool {
        match crate::capabilities::snapshot(cx).get("audio_playback") {
            Some(status) => status.supported,
            None => !cfg!(target_family = "wasm"),
        }
    }
}

impl Render for AudioPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = crate::i18n::localize("audio_title");
        let subtitle = crate::i18n::localize("audio_subtitle");
        let loaded = self.engine.read(cx).status().is_some();

        let content = if !self.playback_supported(cx) {
            self.render_unsupported().into_any_element()
        } else {
            let mut column = v_flex().gap_4();
            #[cfg(not(target_family = "wasm"))]
            {
                column = column.child(self.render_load_row(cx));
            }
            if loaded {
                column = column.child(self.render_meta_line(cx));
            } else {
                column = column.child(self.render_empty());
            }
            column
                .child(self.render_overview(window, cx))
                .child(self.render_transport(cx))
                .child(self.render_live(cx))
                .into_any_element()
        };

        v_flex()
            .id("audio-page")
            .min_h_full()
            .p_6()
            .gap_4()
            .key_context("AudioPage")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &TogglePlayback, _, cx| {
                this.engine.update(cx, |engine, cx| engine.toggle_play(cx));
            }))
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .id("audio-title")
                            .a11y(Role::Heading, title.clone())
                            .aria_level(1)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(subtitle),
                    ),
            )
            .children(self.render_error_alert(cx))
            .child(content)
    }
}

#[cfg(test)]
#[path = "audio.test.rs"]
mod audio_test;
