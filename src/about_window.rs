use gpui::*;
use gpui_kit::component::button::Button;
use gpui_kit::component::TitleBar;

pub struct AboutWindow {
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl AboutWindow {
    pub fn new(
        _cx: &mut Context<Self>,
        on_close: Box<dyn Fn(&mut Window, &mut App) + 'static>,
    ) -> Self {
        Self { on_close: Some(on_close) }
    }
}

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let on_close = self.on_close.take();
        div()
            .size_full()
            .flex()
            .flex_col()
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
                            .on_click(move |_, window, cx| {
                                if let Some(cb) = &on_close {
                                    cb(window, cx);
                                }
                                window.remove_window();
                            })
                    )
            )
    }
}