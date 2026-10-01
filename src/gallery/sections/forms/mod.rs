//! Form-area sections, ported from the upstream `FormStory`,
//! `SettingsStory`, `QuestionnaireStory`, and the tested consumer recipes in
//! the `ai_recipes` example: Form, Settings, Questionnaire, and Recipes.

mod demo;
mod form;
mod questionnaire;
mod recipes;
mod settings;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    form::register(sections, window, cx);
    settings::register(sections, window, cx);
    questionnaire::register(sections, window, cx);
    recipes::register(sections, window, cx);
}
