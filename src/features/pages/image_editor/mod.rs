//! Removable image editor page. `ImageEditorPage` is the only public surface
//! via `views::ImageEditorPage`; everything else stays private to the module.
//! Removal = delete this directory and revert its seam touches: the `image`
//! dependency in Cargo.toml (+ Cargo.lock), the `mod image_editor;`/`pub use`
//! lines in features/pages/mod.rs, the Page::ImageEditor arms and all() entry
//! in shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the image_editor capability in app/init.rs, and
//! the nav_image_editor + image_editor_* keys in locales/en.yml and
//! locales/zh-CN.yml.

mod bake;
mod document;
mod history;
mod tools;

#[cfg(not(target_family = "wasm"))]
mod io;

#[cfg(test)]
#[path = "mod.test.rs"]
mod image_editor_test;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, ElementExt as _, Icon, IconName, Selectable as _,
    Sizable as _, h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::{Canvas, CanvasGrid, CanvasShape, CanvasState, CanvasTool, StrokeStyle};
use document::{FilterSettings, ImageEditorDocument, ImageEditorEvent};
use tools::{ArrowTool, BrushTool, CropTool, EllipseTool, EraserTool, RectTool};

const BRUSH_SVG: &[u8] = include_bytes!("assets/brush.svg");
const ERASER_SVG: &[u8] = include_bytes!("assets/eraser.svg");
const CIRCLE_SVG: &[u8] = include_bytes!("assets/circle.svg");
const ARROW_SVG: &[u8] = include_bytes!("assets/move-up-right.svg");
const CROP_SVG: &[u8] = include_bytes!("assets/crop.svg");
const SAVE_SVG: &[u8] = include_bytes!("assets/save.svg");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTool {
    Brush,
    Eraser,
    Rect,
    Ellipse,
    Arrow,
    Crop,
}

#[derive(Clone, Debug, PartialEq)]
enum SaveState {
    Idle,
    Busy,
    Done,
    Failed(String),
}

pub struct ImageEditorPage {
    canvas: Entity<CanvasState>,
    doc: Entity<ImageEditorDocument>,
    viewport: Bounds<Pixels>,
    pending_fit: bool,
    tool: EditorTool,
    gray: bool,
    color_value: Hsla,
    brush_value: f32,
    eraser_value: f32,
    brush_size: Entity<SliderState>,
    eraser_size: Entity<SliderState>,
    blur: Entity<SliderState>,
    brightness: Entity<SliderState>,
    color: Entity<ColorPickerState>,
    save_state: SaveState,
    reported_error: Option<String>,
    _subscriptions: Vec<Subscription>,
    _filter_subscriptions: Vec<Subscription>,
    _open_task: Option<Task<()>>,
    _save_task: Option<Task<()>>,
}

impl ImageEditorPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|_| CanvasState::new());
        let doc = cx.new(|_| ImageEditorDocument::new());
        canvas.update(cx, |state, cx| {
            state.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            )
        });

        let brush_size = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(100.)
                .step(1.)
                .default_value(4.)
        });
        let eraser_size = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(100.)
                .step(1.)
                .default_value(12.)
        });
        let blur = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(20.)
                .step(0.5)
                .default_value(0.)
        });
        let brightness = cx.new(|_| {
            SliderState::new()
                .min(-100.)
                .max(100.)
                .step(1.)
                .default_value(0.)
        });
        let color = cx.new(|cx| ColorPickerState::new(window, cx));
        let default_color = hsla(0., 1., 0.5, 1.);
        color.update(cx, |state, cx| state.set_value(default_color, window, cx));

        let subscriptions = vec![
            cx.observe(&canvas, |_, _, cx| cx.notify()),
            cx.observe(&doc, |this, entity, cx| {
                let error = entity.read(cx).last_error().map(str::to_string);
                if error != this.reported_error {
                    this.reported_error = error.clone();
                    if let Some(message) = &error {
                        this.report_open_failure(message, cx);
                    }
                }
                cx.notify();
            }),
            cx.subscribe(&doc, |this, _, event: &ImageEditorEvent, cx| match event {
                ImageEditorEvent::ImageOpened => {
                    this.pending_fit = true;
                    this.try_fit(cx);
                    this.mount_active_tool(cx);
                    this.reset_filter_controls(cx);
                }
                ImageEditorEvent::DisplayChanged => this.update_base_shape(cx),
                ImageEditorEvent::HistoryChanged | ImageEditorEvent::CropRectChanged => {}
            }),
            cx.subscribe(&brush_size, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Release(value) = event {
                    this.brush_value = value.start();
                    this.mount_active_tool(cx);
                }
            }),
            cx.subscribe(&eraser_size, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Release(value) = event {
                    this.eraser_value = value.start();
                    this.mount_active_tool(cx);
                }
            }),
            cx.subscribe(&color, |this, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    this.color_value = *color;
                    this.mount_active_tool(cx);
                }
            }),
        ];
        let filter_subscriptions = vec![
            cx.subscribe(&blur, |this, _, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Release(_)) {
                    this.apply_filters(cx);
                }
            }),
            cx.subscribe(&brightness, |this, _, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Release(_)) {
                    this.apply_filters(cx);
                }
            }),
        ];

        Self {
            canvas,
            doc,
            viewport: Bounds::default(),
            pending_fit: false,
            tool: EditorTool::Brush,
            gray: false,
            color_value: default_color,
            brush_value: 4.,
            eraser_value: 12.,
            brush_size,
            eraser_size,
            blur,
            brightness,
            color,
            save_state: SaveState::Idle,
            reported_error: None,
            _subscriptions: subscriptions,
            _filter_subscriptions: filter_subscriptions,
            _open_task: None,
            _save_task: None,
        }
    }

    fn reset_filter_controls(&mut self, cx: &mut Context<Self>) {
        self.gray = false;
        self.blur = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(20.)
                .step(0.5)
                .default_value(0.)
        });
        self.brightness = cx.new(|_| {
            SliderState::new()
                .min(-100.)
                .max(100.)
                .step(1.)
                .default_value(0.)
        });
        self._filter_subscriptions = vec![
            cx.subscribe(&self.blur, |this, _, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Release(_)) {
                    this.apply_filters(cx);
                }
            }),
            cx.subscribe(&self.brightness, |this, _, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Release(_)) {
                    this.apply_filters(cx);
                }
            }),
        ];
    }

    fn select_tool(&mut self, tool: EditorTool, cx: &mut Context<Self>) {
        self.tool = tool;
        self.mount_active_tool(cx);
    }

    fn mount_active_tool(&mut self, cx: &mut Context<Self>) {
        let Some(layer) = self.doc.read(cx).annotation_layer() else {
            return;
        };
        let doc = self.doc.clone();
        let stroke = StrokeStyle {
            width: px(self.brush_value),
            color: self.color_value,
            dash: None,
        };
        let eraser_radius = self.eraser_value;
        let crop_color = cx.theme().foreground;
        let tool: Box<dyn CanvasTool> = match self.tool {
            EditorTool::Brush => Box::new(BrushTool::new(doc, layer, stroke)),
            EditorTool::Eraser => Box::new(EraserTool::new(doc, layer, eraser_radius)),
            EditorTool::Rect => Box::new(RectTool::new(doc, layer, stroke)),
            EditorTool::Ellipse => Box::new(EllipseTool::new(doc, layer, stroke)),
            EditorTool::Arrow => Box::new(ArrowTool::new(doc, layer, stroke)),
            EditorTool::Crop => Box::new(CropTool::new(doc, crop_color)),
        };
        self.canvas.update(cx, |state, cx| state.set_tool(tool, cx));
        cx.notify();
    }

    fn try_fit(&mut self, cx: &mut Context<Self>) {
        if !self.pending_fit || self.viewport.size.width <= px(0.) {
            return;
        }
        let Some((w, h)) = self.doc.read(cx).base_size() else {
            return;
        };
        let viewport = self.viewport;
        self.pending_fit = false;
        self.canvas.update(cx, |state, cx| {
            state.zoom_to_fit(
                Bounds {
                    origin: Point::default(),
                    size: size(px(w as f32), px(h as f32)),
                },
                viewport.size,
                cx,
            )
        });
    }

    fn store_viewport(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        if bounds == self.viewport {
            return;
        }
        self.viewport = bounds;
        if self.pending_fit {
            self.try_fit(cx);
        }
    }

    fn update_base_shape(&mut self, cx: &mut Context<Self>) {
        let (display, base_shape, base_size) = {
            let doc = self.doc.read(cx);
            (doc.display().cloned(), doc.base_shape(), doc.base_size())
        };
        let (Some(display), Some(shape_id), Some((w, h))) = (display, base_shape, base_size) else {
            return;
        };
        let bounds = Bounds {
            origin: Point::default(),
            size: size(px(w as f32), px(h as f32)),
        };
        self.canvas.update(cx, |state, cx| {
            state.edit_document(cx, |doc| {
                doc.update_shape(shape_id, |shape| {
                    *shape = CanvasShape::image(bounds, display.clone());
                });
            })
        });
    }

    fn apply_filters(&mut self, cx: &mut Context<Self>) {
        let blur_sigma = self.blur.read(cx).value().start();
        let brightness = self.brightness.read(cx).value().start() as i32;
        let filters = FilterSettings {
            grayscale: self.gray,
            blur_sigma,
            brightness,
        };
        self.doc.update(cx, |doc, cx| doc.set_filters(filters, cx));
    }

    fn on_undo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let canvas = self.canvas.clone();
        self.doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    }

    fn on_redo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let canvas = self.canvas.clone();
        self.doc.update(cx, |doc, cx| doc.redo(&canvas, cx));
    }

    fn on_rotate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let canvas = self.canvas.clone();
        self.doc.update(cx, |doc, cx| doc.rotate90(&canvas, cx));
    }

    fn on_apply_crop(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let canvas = self.canvas.clone();
        self.doc.update(cx, |doc, cx| doc.apply_crop(&canvas, cx));
    }

    fn on_open(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.save_state = SaveState::Idle;
        let canvas = self.canvas.clone();
        self._open_task = Some(cx.spawn(async move |this, cx| {
            // The prompt must resolve inside this task; a dialog callback that
            // re-enters the app mid-turn panics (RCA §5.2.1).
            let Some(path) = crate::desktop_actions::pick_file(cx).await else {
                return;
            };
            let bytes = cx
                .background_spawn(async move { std::fs::read(&path).map_err(|e| e.to_string()) })
                .await;
            match bytes {
                Ok(bytes) => {
                    this.update(cx, |page, cx| {
                        page.doc.update(cx, |doc, cx| doc.open(bytes, &canvas, cx))
                    })
                    .ok();
                }
                Err(message) => {
                    this.update(cx, |page, cx| page.report_open_failure(&message, cx))
                        .ok();
                }
            }
        }));
    }

    #[cfg(not(target_family = "wasm"))]
    fn on_save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.save_state, SaveState::Busy) || !self.doc.read(cx).has_image() {
            return;
        }
        self.save_state = SaveState::Idle;
        let Some(base) = self.doc.read(cx).base().cloned() else {
            return;
        };
        let filters = self.doc.read(cx).filters();
        let annotations = self.doc.read(cx).annotation_shapes();
        self._save_task = Some(cx.spawn(async move |this, cx| {
            let Some(path) = crate::desktop_actions::save_file(cx).await else {
                return;
            };
            this.update(cx, |page, cx| {
                page.save_state = SaveState::Busy;
                cx.notify();
            })
            .ok();
            let img = cx
                .background_spawn(
                    async move { bake::compose_export(&base, &filters, &annotations) },
                )
                .await;
            let result = cx
                .background_spawn(async move { io::save_image(&path, &img) })
                .await;
            this.update(cx, |page, cx| {
                page.save_state = match result {
                    Ok(()) => SaveState::Done,
                    Err(err) => SaveState::Failed(err.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn report_open_failure(&mut self, message: &str, cx: &mut Context<Self>) {
        crate::error_surface::report(
            crate::i18n::localize_with_args("image_editor_open_failed", &[("error", message)]),
            crate::errors::AppErrorSeverity::Warning,
            crate::error_surface::ErrorCategory::System,
            vec![crate::error_surface::ErrorAction::Dismiss],
            cx,
        );
        cx.notify();
    }

    fn rail_button(
        &self,
        id: &'static str,
        icon: Icon,
        label_key: &'static str,
        tool: EditorTool,
        cx: &mut Context<Self>,
    ) -> Button {
        let label = crate::i18n::localize(label_key);
        Button::new(id)
            .ghost()
            .small()
            .icon(icon)
            .tooltip(label.clone())
            .accessibility_label(label)
            .selected(self.tool == tool)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select_tool(tool, cx)))
    }
}

fn labeled_slider(
    id: &'static str,
    label_key: &'static str,
    state: &Entity<SliderState>,
    disabled: bool,
    label_color: Hsla,
) -> impl IntoElement {
    v_flex()
        .id(id)
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(label_color)
                .child(crate::i18n::localize(label_key)),
        )
        .child(Slider::new(state).disabled(disabled))
}

impl Render for ImageEditorPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity();
        let (has_image, loading, can_undo, can_redo, has_valid_crop, last_error) = {
            let doc = self.doc.read(cx);
            (
                doc.has_image(),
                doc.is_loading(),
                doc.can_undo(),
                doc.can_redo(),
                doc.has_valid_crop(),
                doc.last_error().map(str::to_string),
            )
        };

        #[cfg(target_family = "wasm")]
        let open_button = {
            let reason = crate::i18n::localize("image_editor_open_unavailable");
            Button::new("image-editor-open")
                .label(crate::i18n::localize("image_editor_open"))
                .icon(IconName::FolderOpen)
                .disabled(true)
                .tooltip(reason)
        };
        #[cfg(not(target_family = "wasm"))]
        let open_button = Button::new("image-editor-open")
            .label(crate::i18n::localize("image_editor_open"))
            .icon(IconName::FolderOpen)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.on_open(window, cx)));

        #[cfg(target_family = "wasm")]
        let save_button = {
            let reason = crate::i18n::localize("image_editor_open_unavailable");
            Button::new("image-editor-save")
                .label(crate::i18n::localize("image_editor_save_as"))
                .icon(Icon::default().data(SAVE_SVG))
                .disabled(true)
                .tooltip(reason)
        };
        #[cfg(not(target_family = "wasm"))]
        let save_button = Button::new("image-editor-save")
            .label(crate::i18n::localize("image_editor_save_as"))
            .icon(Icon::default().data(SAVE_SVG))
            .disabled(!has_image || loading || matches!(self.save_state, SaveState::Busy))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.on_save(window, cx)));

        let undo_label = crate::i18n::localize("image_editor_undo");
        let undo_button = Button::new("image-editor-undo")
            .ghost()
            .small()
            .icon(IconName::Undo2)
            .tooltip(undo_label.clone())
            .accessibility_label(undo_label)
            .disabled(!can_undo)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.on_undo(window, cx)));
        let redo_label = crate::i18n::localize("image_editor_redo");
        let redo_button = Button::new("image-editor-redo")
            .ghost()
            .small()
            .icon(IconName::Redo2)
            .tooltip(redo_label.clone())
            .accessibility_label(redo_label)
            .disabled(!can_redo)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.on_redo(window, cx)));
        let rotate_button = Button::new("image-editor-rotate")
            .label(crate::i18n::localize("image_editor_rotate"))
            .icon(IconName::RotateCw)
            .disabled(!has_image || loading)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.on_rotate(window, cx)));
        let crop_button = Button::new("image-editor-apply-crop")
            .label(crate::i18n::localize("image_editor_crop_apply"))
            .icon(Icon::default().data(CROP_SVG))
            .disabled(!has_valid_crop)
            .on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.on_apply_crop(window, cx)),
            );

        let save_status = match &self.save_state {
            SaveState::Idle => div().into_any_element(),
            SaveState::Busy => h_flex()
                .id("image-editor-save-status")
                .a11y_live(accesskit::Live::Polite)
                .gap_1()
                .items_center()
                .child(Spinner::new().xsmall())
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(crate::i18n::localize("image_editor_saving")),
                )
                .into_any_element(),
            SaveState::Done => div()
                .id("image-editor-save-status")
                .a11y_live(accesskit::Live::Polite)
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(crate::i18n::localize("image_editor_saved"))
                .into_any_element(),
            SaveState::Failed(err) => div()
                .id("image-editor-save-status")
                .a11y_live(accesskit::Live::Polite)
                .text_sm()
                .text_color(cx.theme().danger)
                .child(crate::i18n::localize_with_args(
                    "image_editor_save_failed",
                    &[("error", err.as_str())],
                ))
                .into_any_element(),
        };

        let toolbar = h_flex()
            .id("image-editor-toolbar")
            .border_b_1()
            .border_color(cx.theme().border)
            .px_4()
            .py_2()
            .gap_2()
            .items_center()
            .flex_wrap()
            .child(open_button)
            .child(save_button)
            .child(save_status)
            .child(undo_button)
            .child(redo_button)
            .child(rotate_button)
            .child(crop_button)
            .when_some(last_error, |row, error| {
                row.child(
                    div()
                        .id("image-editor-open-error")
                        .a11y_live(accesskit::Live::Polite)
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .min_w_0()
                        .truncate()
                        .child(error),
                )
            });

        let rail = v_flex()
            .id("image-editor-rail")
            .border_r_1()
            .border_color(cx.theme().border)
            .p_2()
            .gap_1()
            .flex_shrink_0()
            .child(self.rail_button(
                "image-editor-tool-brush",
                Icon::default().data(BRUSH_SVG),
                "image_editor_tool_brush",
                EditorTool::Brush,
                cx,
            ))
            .child(self.rail_button(
                "image-editor-tool-eraser",
                Icon::default().data(ERASER_SVG),
                "image_editor_tool_eraser",
                EditorTool::Eraser,
                cx,
            ))
            .child(self.rail_button(
                "image-editor-tool-rect",
                Icon::new(IconName::Square),
                "image_editor_tool_rect",
                EditorTool::Rect,
                cx,
            ))
            .child(self.rail_button(
                "image-editor-tool-ellipse",
                Icon::default().data(CIRCLE_SVG),
                "image_editor_tool_ellipse",
                EditorTool::Ellipse,
                cx,
            ))
            .child(self.rail_button(
                "image-editor-tool-arrow",
                Icon::default().data(ARROW_SVG),
                "image_editor_tool_arrow",
                EditorTool::Arrow,
                cx,
            ))
            .child(self.rail_button(
                "image-editor-tool-crop",
                Icon::default().data(CROP_SVG),
                "image_editor_tool_crop",
                EditorTool::Crop,
                cx,
            ));

        let canvas_area = div()
            .id("image-editor-canvas")
            .a11y(
                Role::Group,
                crate::i18n::localize("image_editor_canvas_label"),
            )
            .relative()
            .flex_1()
            .min_w_0()
            .on_prepaint({
                let page = page.clone();
                move |bounds, _, cx| {
                    page.update(cx, |this, cx| this.store_viewport(bounds, cx));
                }
            })
            .child(Canvas::new("image-editor-canvas", self.canvas.clone()))
            .when(!has_image, |area| {
                area.child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(cx.theme().background)
                        .items_center()
                        .justify_center()
                        .child(
                            v_flex()
                                .id("image-editor-empty")
                                .gap_2()
                                .items_center()
                                .child(
                                    div()
                                        .id("image-editor-title")
                                        .a11y(
                                            Role::Heading,
                                            crate::i18n::localize("image_editor_title"),
                                        )
                                        .aria_level(1)
                                        .text_xl()
                                        .font_weight(FontWeight::BOLD)
                                        .child(crate::i18n::localize("image_editor_title")),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(crate::i18n::localize("image_editor_empty_hint")),
                                ),
                        ),
                )
            });

        let panel_switch = {
            let page = page.clone();
            Switch::new("image-editor-grayscale")
                .checked(self.gray)
                .label(crate::i18n::localize("image_editor_filter_grayscale"))
                .disabled(!has_image || loading)
                .on_click(move |checked: &bool, _, cx| {
                    page.update(cx, |this, cx| {
                        this.gray = *checked;
                        this.apply_filters(cx);
                    });
                })
        };

        let filter_panel = v_flex()
            .id("image-editor-panel")
            .border_l_1()
            .border_color(cx.theme().border)
            .w_64()
            .flex_shrink_0()
            .p_4()
            .gap_4()
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        ColorPicker::new(&self.color)
                            .label(crate::i18n::localize("image_editor_color")),
                    )
                    .child(labeled_slider(
                        "image-editor-brush-size",
                        "image_editor_brush_size",
                        &self.brush_size,
                        false,
                        cx.theme().foreground,
                    ))
                    .child(labeled_slider(
                        "image-editor-eraser-size",
                        "image_editor_eraser_size",
                        &self.eraser_size,
                        false,
                        cx.theme().foreground,
                    )),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(crate::i18n::localize("image_editor_filters")),
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(panel_switch)
                    .child(labeled_slider(
                        "image-editor-blur",
                        "image_editor_filter_blur",
                        &self.blur,
                        !has_image || loading,
                        cx.theme().foreground,
                    ))
                    .child(labeled_slider(
                        "image-editor-brightness",
                        "image_editor_filter_brightness",
                        &self.brightness,
                        !has_image || loading,
                        cx.theme().foreground,
                    )),
            );

        v_flex()
            .id("image-editor")
            .size_full()
            .min_h_0()
            .child(toolbar)
            .child(
                h_flex()
                    .id("image-editor-body")
                    .flex_1()
                    .min_h_0()
                    .child(rail)
                    .child(canvas_area)
                    .child(filter_panel),
            )
    }
}
