use std::path::PathBuf;
use ::gpui::*;
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::TitleBar;
use gpui_component::{ActiveTheme, GlobalState, WindowExt};
use gpui_kit::component::{Theme, ThemeMode};
use twrite::{Editor, focus_editor, EditorTheme};
use ::gpui::PathChoice;

mod about_window;
mod settings;
use about_window::{AboutWindow, CloseAbout};
use settings::ThemeKind;

// --- Действия ---
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Quit;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct About;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct SetLightTheme;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct SetDarkTheme;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct NewFile;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct OpenFile;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct SaveFile;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct SaveFileAs;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Undo;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Redo;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Cut;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Copy;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct Paste;
#[derive(Clone, PartialEq, Default, Debug, gpui_kit::Action)]
#[action(namespace = my_editor, no_json)]
struct SelectAll;

// --- Главный вид ---
struct RootView {
    menu_bar: Entity<AppMenuBar>,
    editor: Entity<Editor>,
    current_file: Option<PathBuf>,
}

fn light_editor_theme() -> EditorTheme {
    let mut t = EditorTheme::default();
    t.background = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xfafafa));
    t.foreground = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x1f1f1f));
    t.cursor = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x0969da));
    t.selection = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xcce5ff));
    t.search_match = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xfff3c4));
    t.line_number = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x8a8a8a));
    t.line_number_active = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x1f1f1f));
    t.menu_bg = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xffffff));
    t.menu_border = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xd0d7de));
    t.menu_hover = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0xeff2f5));
    t.menu_fg = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x1f1f1f));
    t.menu_hint = EditorTheme::rgba_to_hsla(twrite::Rgba::hex(0x6e7781));
    t
}

fn dark_editor_theme() -> EditorTheme {
    EditorTheme::default()
}

fn editor_theme_for(kind: ThemeKind) -> EditorTheme {
    match kind {
        ThemeKind::Light => light_editor_theme(),
        ThemeKind::Dark => dark_editor_theme(),
    }
}

fn apply_theme(kind: ThemeKind, editor: &Entity<Editor>, cx: &mut App) {
    match kind {
        ThemeKind::Light => Theme::change(ThemeMode::Light, None, cx),
        ThemeKind::Dark => Theme::change(ThemeMode::Dark, None, cx),
    }
    editor.update(cx, |ed, cx| {
        ed.theme = editor_theme_for(kind);
        cx.notify();
    });
}

impl RootView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let menu_bar = AppMenuBar::new(cx);

        let start_kind = settings::load_theme().unwrap_or(ThemeKind::Dark);
        let editor = cx.new(|cx| {
            let mut ed = Editor::new("Hello from TWrite + gpui-kit + menu!", cx);
            ed.theme = editor_theme_for(start_kind);
            ed
        });

        let focus_handle = editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
        Self { menu_bar, editor, current_file: None }
    }

    fn get_text(&self, cx: &App) -> String {
        self.editor.read(cx).buffer.text().to_string()
    }

    fn set_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            let len = ed.buffer.len_bytes();
            ed.buffer.delete_range(0..len);
            ed.buffer.insert(&text);
            cx.notify();
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_new(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.current_file = None;
        self.set_text(String::new(), window, cx);
    }

    fn on_open(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Открыть файл".into()),
            choices: vec![PathChoice {
                id: "encoding".into(),
                label: "Кодировка".into(),
                options: vec![
                    ("utf-8".into(), "UTF-8".into()),
                    ("windows-1251".into(), "CP-1251".into()),
                    ("ibm866".into(), "CP-866".into()),
                ],
                initial: "utf-8".into(),
            }],
            ..Default::default()
        });
        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some((mut paths, choices)))) => {
                if let Some(path) = paths.pop() {
                    let encoding = choices
                        .iter()
                        .find(|(k, _)| k == "encoding")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "utf-8".to_string());
                    _ = this.update_in(cx, |this, window, cx| {
                        this.open_file_with_encoding(path, encoding, window, cx);
                    });
                }
            }
            Ok(Err(err)) => println!("Ошибка выбора файла: {}", err),
            _ => {}
        })
        .detach();
    }

    fn open_file_with_encoding(
        &mut self,
        path: PathBuf,
        encoding: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                use gpui_kit::component::notification::Notification;
                window.push_notification(
                    Notification::error(format!("Ошибка чтения файла: {}", e)),
                    cx,
                );
                return;
            }
        };

        let encoding_ref = encoding_rs::Encoding::for_label(encoding.as_bytes())
            .unwrap_or(encoding_rs::UTF_8);
        let (decoded, _, had_errors) = encoding_ref.decode(&bytes);
        if had_errors {
            use gpui_kit::component::notification::Notification;
            window.push_notification(
                Notification::error(format!(
                    "Не удалось декодировать файл как {}",
                    encoding
                )),
                cx,
            );
            return;
        }

        self.editor.update(cx, |ed, cx| {
            let text = decoded.into_owned();
            let len = ed.buffer.len_bytes();
            ed.buffer.delete_range(0..len);
            ed.buffer.insert(&text);
            cx.notify();
        });
        self.current_file = Some(path);
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_save(&mut self, _: &SaveFile, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = &self.current_file {
            let text = self.get_text(cx);
            if let Err(e) = std::fs::write(path, text) {
                println!("Ошибка сохранения: {}", e);
            } else {
                println!("Сохранено: {:?}", path);
            }
        } else {
            self.on_save_as(&SaveFileAs, window, cx);
        }
    }

    fn on_save_as(&mut self, _: &SaveFileAs, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Сохранить как".into()),
            ..Default::default()
        });
        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some((mut paths, _choices)))) => {
                if let Some(path) = paths.pop() {
                    _ = this.update_in(cx, |this, _window, cx| {
                        let text = this.get_text(cx);
                        if let Err(e) = std::fs::write(&path, text) {
                            println!("Ошибка сохранения: {}", e);
                        } else {
                            this.current_file = Some(path);
                            println!("Сохранено как: {:?}", this.current_file);
                        }
                    });
                }
            }
            Ok(Err(err)) => println!("Ошибка выбора файла: {}", err),
            _ => {}
        })
        .detach();
    }

    // --- Правка через TWrite ---
    fn on_undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            ed.buffer.undo_word();
            cx.notify();
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            ed.buffer.redo_word();
            cx.notify();
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            if ed.cut(cx) {
                cx.notify();
            }
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.read(cx).copy(cx);
    }

    fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            if ed.paste(cx) {
                cx.notify();
            }
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |ed, cx| {
            let len = ed.buffer.len_bytes();
            ed.selection = Some(twrite::Selection::range(0, len));
            cx.notify();
        });
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_set_light_theme(&mut self, _: &SetLightTheme, _window: &mut Window, cx: &mut Context<Self>) {
        apply_theme(ThemeKind::Light, &self.editor, cx);
        settings::save_theme(ThemeKind::Light);
        cx.notify();
    }

    fn on_set_dark_theme(&mut self, _: &SetDarkTheme, _window: &mut Window, cx: &mut Context<Self>) {
        apply_theme(ThemeKind::Dark, &self.editor, cx);
        settings::save_theme(ThemeKind::Dark);
        cx.notify();
    }

    fn on_about(&mut self, _: &About, _window: &mut Window, cx: &mut Context<Self>) {
        let bounds = Bounds::centered(None, size(px(400.), px(200.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            kind: WindowKind::Dialog,
            window_decorations: Some(WindowDecorations::Client),
            titlebar: Some(TitleBar::title_bar_options()),
            app_owns_titlebar_drag: true,
            window_min_size: Some(size(px(300.), px(150.))),
            ..Default::default()
        };

        let root_weak = cx.entity().downgrade();
        gpui_kit::open_window(options, cx, move |window, cx| {
            let root_weak_cb = root_weak.clone();
            let on_close: Box<dyn Fn(&mut Window, &mut App) + 'static> =
                Box::new(move |_w, app| {
                    let _ = root_weak_cb.update(app, |_root, _cx| {});
                });
            cx.new(|cx| AboutWindow::new(window, cx, on_close))
        })
        .ok();
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::on_new))
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_set_light_theme))
            .on_action(cx.listener(Self::on_set_dark_theme))
            .on_action(cx.listener(Self::on_about))
            .child(
                TitleBar::new()
                    .h(px(32.))
                    .child(
                        div().flex().flex_1().items_center().justify_center().child(
                            self.current_file.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "Редактор TWrite".to_string())
                        )
                    )
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_2()
                    .h(px(30.))
                    .bg(cx.theme().background)
                    .border_b_1()
                    .child(self.menu_bar.clone())
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .h_full()
                    .child(self.editor.clone())
            )
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);

            let start_theme = settings::load_theme().unwrap_or(ThemeKind::Dark);
            Theme::change(
                match start_theme {
                    ThemeKind::Light => ThemeMode::Light,
                    ThemeKind::Dark => ThemeMode::Dark,
                },
                None,
                cx,
            );

            cx.bind_keys([
                KeyBinding::new("secondary-s", SaveFile, None),
                KeyBinding::new("secondary-shift-s", SaveFileAs, None),
                KeyBinding::new("secondary-z", Undo, None),
                KeyBinding::new("secondary-shift-z", Redo, None),
                KeyBinding::new("secondary-x", Cut, None),
                KeyBinding::new("secondary-c", Copy, None),
                KeyBinding::new("secondary-v", Paste, None),
                KeyBinding::new("secondary-a", SelectAll, None),
                KeyBinding::new("secondary-n", NewFile, None),
                KeyBinding::new("secondary-o", OpenFile, None),
                KeyBinding::new("escape", CloseAbout, Some("AboutWindow")),
            ]);

            let menus = vec![
                gpui_kit::Menu {
                    name: SharedString::new_static("Файл"),
                    disabled: false,
                    items: vec![
                        gpui_kit::MenuItem::action("Новый", NewFile),
                        gpui_kit::MenuItem::action("Открыть...", OpenFile),
                        gpui_kit::MenuItem::separator(),
                        gpui_kit::MenuItem::action("Сохранить", SaveFile),
                        gpui_kit::MenuItem::action("Сохранить как...", SaveFileAs),
                        gpui_kit::MenuItem::separator(),
                        gpui_kit::MenuItem::action("Выход", Quit),
                    ],
                },
                gpui_kit::Menu {
                    name: SharedString::new_static("Правка"),
                    disabled: false,
                    items: vec![
                        gpui_kit::MenuItem::action("Отменить", Undo),
                        gpui_kit::MenuItem::action("Повторить", Redo),
                        gpui_kit::MenuItem::separator(),
                        gpui_kit::MenuItem::action("Вырезать", Cut),
                        gpui_kit::MenuItem::action("Копировать", Copy),
                        gpui_kit::MenuItem::action("Вставить", Paste),
                        gpui_kit::MenuItem::separator(),
                        gpui_kit::MenuItem::action("Выделить всё", SelectAll),
                    ],
                },
                gpui_kit::Menu {
                    name: SharedString::new_static("Справка"),
                    disabled: false,
                    items: vec![
                        gpui_kit::MenuItem::action("Светлая тема", SetLightTheme),
                        gpui_kit::MenuItem::action("Тёмная тема", SetDarkTheme),
                        gpui_kit::MenuItem::separator(),
                        gpui_kit::MenuItem::action("О программе", About),
                    ],
                },
            ];
            let owned_menus: Vec<_> = menus.into_iter().map(|m| m.owned()).collect();
            GlobalState::global_mut(cx).set_app_menus(owned_menus);

            cx.on_action(|_: &Quit, cx| cx.quit());

            let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_decorations: Some(WindowDecorations::Client),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(size(px(400.), px(300.))),
                ..Default::default()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| RootView::new(window, cx))
            })
            .expect("Не удалось открыть окно");

            cx.activate(true);
        });
}