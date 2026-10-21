use std::path::PathBuf;
use ::gpui::*;
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::component::TitleBar;
use gpui_component::{ActiveTheme, GlobalState, WindowExt};
use gpui_kit::component::{Theme, ThemeMode};
use twrite::{Editor, focus_editor, EditorTheme};
use ::gpui::PathChoice;

mod about_window;
mod save_dialog;
mod settings;
use about_window::{AboutWindow, CloseAbout};
use save_dialog::{CloseSaveDialog, SaveDialog};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingAction { New, Open }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SaveResult { ProceedToAction, DiscardAndProceed }
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
    saved_version: usize,
    quit_after_save: bool,
    pending_action: Option<PendingAction>,
    pending_save_result: Option<SaveResult>,
    root_window_handle: Option<gpui::AnyWindowHandle>,
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
        let saved_version = editor.read(cx).buffer.version();

        // Перехватываем системное закрытие окна (крестик, Alt+F4).
        // Если есть несохранённые изменения — показываем SaveDialog и НЕ закрываем окно.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let Some(root) = this.upgrade() else { return true };
            let dirty = root.read(cx).is_dirty(cx);
            if dirty {
                root.update(cx, |root, cx| {
                    root.request_quit(window, cx);
                });
                false // не закрывать окно — диалог уже открыт
            } else {
                true // закрыть окно
            }
        });
        

        Self {
            menu_bar,
            editor,
            current_file: None,
            saved_version,
            quit_after_save: false,
            pending_action: None,
            pending_save_result: None,
            root_window_handle: None,
        }
    }

    fn is_dirty(&self, cx: &App) -> bool {
        self.editor.read(cx).buffer.version() != self.saved_version
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

    /// Единая точка выхода: открывает отдельное окно SaveDialog.
    fn request_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request_save_dialog(None, window, cx);
    }

    /// Открывает SaveDialog; после выбора — либо квит, либо pending_action.
    fn request_save_dialog(
        &mut self,
        action: Option<PendingAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_dirty(cx) {
            match action {
                None => cx.quit(),
                Some(_) => self.run_pending_action(window, cx, action),
            }
            return;
        }
        self.pending_action = action;
        self.quit_after_save = action.is_none();

        let bounds = Bounds::centered(None, size(px(500.), px(250.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            kind: WindowKind::Dialog,
            window_decorations: Some(WindowDecorations::Client),
            titlebar: Some(TitleBar::title_bar_options()),
            app_owns_titlebar_drag: true,
            window_min_size: Some(size(px(350.), px(150.))),
            ..Default::default()
        };

        let root_weak = cx.entity().downgrade();
        gpui_kit::open_window(options, cx, move |window, cx| {
            let root_weak_save = root_weak.clone();
            let root_weak_save_drain = root_weak.clone();
            let root_weak_discard = root_weak.clone();
            let root_weak_discard_drain = root_weak.clone();
            let on_save: Box<dyn Fn(&mut Window, &mut App) + 'static> =
                Box::new(move |_w, app| {
                    // 1. Установить флаг и прочитать handle — один borrow app.
                    let handle_opt = {
                        let mut out = None;
                        let _ = root_weak_save.update(app, |root, cx| {
                            root.pending_save_result = Some(SaveResult::ProceedToAction);
                            out = root.root_window_handle;
                            cx.notify();
                        });
                        out
                    };
                    // 2. Дренаж — app свободен.
                    if let Some(handle) = handle_opt {
                        let weak = root_weak_save_drain.clone();
                        let _ = handle.update(app, |_r, window, app3| {
                            if let Some(e) = weak.upgrade() {
                                let _ = e.update(app3, |root3, cx3| {
                                    if let Some(result) = root3.pending_save_result.take() {
                                        root3.handle_save_result(result, window, cx3);
                                    }
                                });
                            }
                        });
                    }
                });
            let on_discard: Box<dyn Fn(&mut Window, &mut App) + 'static> =
                Box::new(move |_w, app| {
                    let handle_opt = {
                        let mut out = None;
                        let _ = root_weak_discard.update(app, |root, cx| {
                            root.pending_save_result = Some(SaveResult::DiscardAndProceed);
                            out = root.root_window_handle;
                            cx.notify();
                        });
                        out
                    };
                    if let Some(handle) = handle_opt {
                        let weak = root_weak_discard_drain.clone();
                        let _ = handle.update(app, |_r, window, app3| {
                            if let Some(e) = weak.upgrade() {
                                let _ = e.update(app3, |root3, cx3| {
                                    if let Some(result) = root3.pending_save_result.take() {
                                        root3.handle_save_result(result, window, cx3);
                                    }
                                });
                            }
                        });
                    }
                });
            cx.new(|cx| SaveDialog::new(window, cx, on_save, on_discard))
        })
        .ok();
    }

    fn run_pending_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: Option<PendingAction>,
    ) {
        match action {
            Some(PendingAction::New) => self.do_new(window, cx),
            Some(PendingAction::Open) => self.do_open(window, cx),
            None => {}
        }
    }

    fn handle_save_result(
        &mut self,
        result: SaveResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!("handle_save_result: {:?}", result);
        match result {
            SaveResult::ProceedToAction => {
                // "Сохранить": выполняем on_save в контексте редактора.
                // При успехе on_save сам выполнит pending_action/quit.
                // При ошибке push_notification пойдёт в окно редактора.
                self.on_save(&SaveFile, window, cx);
            }
            SaveResult::DiscardAndProceed => {
                self.saved_version = self.editor.read(cx).buffer.version();
                match self.pending_action.take() {
                    Some(a) => self.run_pending_action(window, cx, Some(a)),
                    None => cx.quit(),
                }
            }
        }
    }

    fn on_new(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dirty(cx) {
            self.request_save_dialog(Some(PendingAction::New), window, cx);
            return;
        }
        self.do_new(window, cx);
    }

    fn do_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.current_file = None;
        self.set_text(String::new(), window, cx);
        self.saved_version = self.editor.read(cx).buffer.version();
    }

    fn on_open(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dirty(cx) {
            self.request_save_dialog(Some(PendingAction::Open), window, cx);
            return;
        }
        self.do_open(window, cx);
    }

    fn do_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            Ok(Err(err)) => {
                use gpui_kit::component::notification::Notification;
                _ = this.update_in(cx, |_this, window, cx| {
                    window.push_notification(
                        Notification::error(format!("Ошибка выбора файла: {}", err)),
                        cx,
                    );
                });
            }
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

        // Читаем с выбранной кодировкой (utf-8 / cp1251 / cp866 — см. PathChoice).
        // ВАЖНО: сохранение — всегда в UTF-8 (см. on_save / on_save_as).
        // Это осознанное решение; пользователь в курсе.
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
        self.saved_version = self.editor.read(cx).buffer.version();
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        focus_editor(&focus_handle, window, cx);
    }

    fn on_save(&mut self, _: &SaveFile, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = &self.current_file {
            let text = self.get_text(cx);
            // ВАЖНО: сохраняем всегда в UTF-8 — это осознанное решение.
            // Буфер редактора — String (UTF-8), кодировка при сохранении
            // не выбирается. Пользователь в курсе.
            // (При открытии — кодировка выбирается через PathChoice, см.
            //  open_file_with_encoding; при сохранении — всегда UTF-8.)
            if let Err(e) = std::fs::write(path, text) {
                use gpui_kit::component::notification::Notification;
                window.push_notification(
                    Notification::error(format!("Ошибка сохранения: {}", e)),
                    cx,
                );
                // K1: сохранение не удалось — намерение не выполнено.
                self.pending_action = None;
                self.quit_after_save = false;
            } else {
                self.saved_version = self.editor.read(cx).buffer.version();
                if let Some(a) = self.pending_action {
                    self.pending_action = None;
                    self.run_pending_action(window, cx, Some(a));
                } else if self.quit_after_save {
                    self.quit_after_save = false;
                    cx.quit();
                }
            }
        } else {
            self.on_save_as(&SaveFileAs, window, cx);
        }
    }

    fn on_save_as(&mut self, _: &SaveFileAs, window: &mut Window, cx: &mut Context<Self>) {
        let default_name = self.current_file.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "untitled.txt".to_string());

        let picked = cx.prompt_for_new_path(
            std::path::Path::new("."),
            Some(&default_name),
        );

        cx.spawn_in(window, async move |this, cx| match picked.await {
            Ok(Ok(Some(path))) => {
                _ = this.update_in(cx, |this, _window, cx| {
                    let text = this.get_text(cx);
                    // ВАЖНО: сохраняем всегда в UTF-8 — осознанное решение.
                    // Кодировка при сохранении не выбирается (пользователь в курсе).
                    if let Err(e) = std::fs::write(&path, text) {
                        use gpui_kit::component::notification::Notification;
                        _window.push_notification(
                            Notification::error(format!("Ошибка сохранения: {}", e)),
                            cx,
                        );
                        // K1: сбросить намерение при ошибке записи.
                        this.pending_action = None;
                        this.quit_after_save = false;
                    } else {
                        this.current_file = Some(path);
                        this.saved_version = this.editor.read(cx).buffer.version();
                        if let Some(a) = this.pending_action {
                            this.pending_action = None;
                            this.run_pending_action(_window, cx, Some(a));
                        } else if this.quit_after_save {
                            this.quit_after_save = false;
                            cx.quit();
                        }
                    }
                });
            }
            Ok(Err(err)) => {
                use gpui_kit::component::notification::Notification;
                _ = this.update_in(cx, |this, window, cx| {
                    window.push_notification(
                        Notification::error(format!("Ошибка выбора файла: {}", err)),
                        cx,
                    );
                    // K1: ошибка выбора — сбросить намерение.
                    this.pending_action = None;
                    this.quit_after_save = false;
                });
            }
            _ => {
                // K1: отмена выбора — сбросить намерение.
                _ = this.update_in(cx, |this, _window, _cx| {
                    this.pending_action = None;
                    this.quit_after_save = false;
                });
            }
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

    fn on_quit(&mut self, _: &Quit, window: &mut Window, cx: &mut Context<Self>) {
        self.request_quit(window, cx);
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
        let dirty = self.is_dirty(cx);
        let title = match (&self.current_file, dirty) {
            (Some(p), true) => format!("* {} — Редактор TWrite", p.to_string_lossy()),
            (Some(p), false) => format!("{} — Редактор TWrite", p.to_string_lossy()),
            (None, true) => "* Безымянный — Редактор TWrite".to_string(),
            (None, false) => "Редактор TWrite".to_string(),
        };

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
            .key_context("RootView")
            .on_action(cx.listener(Self::on_quit))
            .on_action(cx.listener(Self::on_about))
            .child(
                TitleBar::new()
                    .h(px(32.))
                    .on_close_window({
                        let this = cx.entity().downgrade();
                        move |_, window, cx| {
                            if let Some(root) = this.upgrade() {
                                root.update(cx, |root, cx| {
                                    root.request_quit(window, cx);
                                });
                            }
                        }
                    })
                    .child(
                        div().flex().flex_1().items_center().justify_center().child(title)
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
                KeyBinding::new("alt-f4", Quit, Some("RootView")),
                KeyBinding::new("alt-f4", CloseSaveDialog, Some("SaveDialog")),
                KeyBinding::new("alt-f4", CloseAbout, Some("AboutWindow")),
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
                KeyBinding::new("escape", CloseSaveDialog, Some("SaveDialog")),
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

            let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_decorations: Some(WindowDecorations::Client),
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(size(px(400.), px(300.))),
                ..Default::default()
            };

            let (root_handle, root_entity) = gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| RootView::new(window, cx))
            })
            .expect("Не удалось открыть окно");
            root_entity.update(cx, |root, cx| {
                root.root_window_handle = Some(root_handle);
                cx.notify();
            });

            cx.activate(true);
        });
}