use gpui::*;
use gpui_kit::component::button::Button;
use gpui_kit::component::TitleBar;

#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
pub struct CloseAbout;

pub struct AboutWindow {
    focus_handle: FocusHandle,
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl AboutWindow {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        on_close: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        // Принудительно ставим фокус на корень окна About, чтобы
        // key_context("AboutWindow") попал в dispatch path и Esc сработал.
        window.focus(&focus_handle, cx);
        Self { focus_handle, on_close: Some(on_close) }
    }

    fn close(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(cb) = self.on_close.take() {
            cb(window, cx);
        }
        window.remove_window();
    }
}

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("AboutWindow")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(|this, _: &CloseAbout, window, cx| {
                this.close(window, cx);
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
                            .child("О программе")
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
                            .child("TWrite Editor v0.1.0")
                    )
                    .child(
                        Button::new("about-ok")
                            .label("ОК")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close(window, cx);
                            }))
                    )
            )
    }
}