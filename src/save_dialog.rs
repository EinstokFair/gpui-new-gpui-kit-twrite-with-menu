use gpui::*;
use gpui_kit::component::button::Button;
use gpui_kit::component::TitleBar;

#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
pub struct CloseSaveDialog;

pub struct SaveDialog {
    focus_handle: FocusHandle,
    on_save: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_discard: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl SaveDialog {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        on_save: Box<dyn Fn(&mut Window, &mut App) + 'static>,
        on_discard: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        Self {
            focus_handle,
            on_save: Some(on_save),
            on_discard: Some(on_discard),
        }
    }

    fn close_window(&self, window: &mut Window) {
        window.remove_window();
    }
}

impl Render for SaveDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("SaveDialog")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_window(window);
                }
            }))
            .on_action(cx.listener(|this, _: &CloseSaveDialog, window, _cx| {
                this.close_window(window);
            }))
            .child(
                TitleBar::new()
                    .h(px(32.))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .child("Несохранённые изменения")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .child(
                        div()
                            .text_base()
                            .child("В документе есть несохранённые изменения. Сохранить перед выходом?")
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                Button::new("save-ok")
                                    .label("Сохранить")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(cb) = this.on_save.take() {
                                            cb(window, cx);
                                        }
                                        this.close_window(window);
                                    }))
                            )
                            .child(
                                Button::new("save-discard")
                                    .label("Не сохранять")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(cb) = this.on_discard.take() {
                                            cb(window, cx);
                                        }
                                        this.close_window(window);
                                    }))
                            )
                            .child(
                                Button::new("save-cancel")
                                    .label("Отмена")
                                    .on_click(cx.listener(|this, _, window, _cx| {
                                        this.close_window(window);
                                    }))
                            )
                    )
            )
    }
}