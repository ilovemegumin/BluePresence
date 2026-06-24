use crate::{config::AppConfig, monitor::MonitorHandle};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, RichText, Stroke};
use std::{fs, sync::Arc, time::Duration};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

const BACKGROUND: Color32 = Color32::from_rgb(24, 26, 30);
const SURFACE: Color32 = Color32::from_rgb(35, 38, 43);
const ACCENT: Color32 = Color32::from_rgb(105, 177, 218);
const TEXT: Color32 = Color32::from_rgb(238, 240, 243);
const MUTED: Color32 = Color32::from_rgb(157, 163, 172);
const ERROR: Color32 = Color32::from_rgb(226, 112, 112);

pub struct BluePresenceApp {
    draft: AppConfig,
    monitor: MonitorHandle,
    tray_icon: Option<TrayIcon>,
    show_menu_id: MenuId,
    quit_menu_id: MenuId,
    feedback: Option<String>,
    first_run: bool,
    quitting: bool,
}

impl BluePresenceApp {
    pub fn new(
        context: &eframe::CreationContext<'_>,
        icon: egui::IconData,
        first_run: bool,
    ) -> Self {
        configfonts(&context.egui_ctx);
        configstyle(&context.egui_ctx);

        let draft = AppConfig::load();
        let monitor = MonitorHandle::start(draft.clone());
        let menu = Menu::new();
        let show_item = MenuItem::new("設定を開く", true, None);
        let quit_item = MenuItem::new("終了", true, None);
        let show_menu_id = show_item.id().clone();
        let quit_menu_id = quit_item.id().clone();
        let _ = menu.append_items(&[&show_item, &quit_item]);
        let tray_icon = Icon::from_rgba(icon.rgba, icon.width, icon.height)
            .ok()
            .and_then(|icon| {
                TrayIconBuilder::new()
                    .with_tooltip("BluePresence")
                    .with_menu(Box::new(menu))
                    .with_icon(icon)
                    .build()
                    .ok()
            });

        Self {
            draft,
            monitor,
            tray_icon,
            show_menu_id,
            quit_menu_id,
            feedback: None,
            first_run,
            quitting: false,
        }
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_menu_id {
                show_window(ctx);
            } else if event.id == self.quit_menu_id {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                show_window(ctx);
            }
        }
    }

    fn save(&mut self) -> bool {
        let normalized = self.draft.clone().normalized();
        if let Err(message) = normalized.validate() {
            self.feedback = Some(message.into());
            return false;
        }
        match normalized.save() {
            Ok(()) => {
                self.draft = normalized.clone();
                self.monitor.update_config(normalized);
                self.first_run = false;
                self.feedback = None;
                true
            }
            Err(error) => {
                self.feedback = Some(format!("保存できません: {error}"));
                false
            }
        }
    }

    fn skip(&mut self) -> bool {
        let empty = AppConfig::default();
        match empty.save() {
            Ok(()) => {
                self.draft = empty.clone();
                self.monitor.update_config(empty);
                self.first_run = false;
                self.feedback = None;
                true
            }
            Err(error) => {
                self.feedback = Some(format!("設定を作成できません: {error}"));
                false
            }
        }
    }
}

impl eframe::App for BluePresenceApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray_events(ctx);
        if ctx.input(|input| input.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        ctx.request_repaint_after(Duration::from_millis(300));
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();

        egui::Panel::bottom("footer")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(BACKGROUND))
            .show_inside(root, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("© 2026 Y2K Development")
                            .size(11.0)
                            .color(MUTED),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show_inside(root, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(if self.first_run {
                                    "初期設定"
                                } else {
                                    "設定"
                                })
                                .size(22.0)
                                .strong()
                                .color(TEXT),
                            );
                        });

                        if self.first_run {
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "Discordに表示する情報を設定してください。設定はスキップできます。",
                                    )
                                    .size(12.0)
                                    .color(MUTED),
                                )
                                .wrap(),
                            );
                        }

                        ui.add_space(18.0);
                        field(
                            ui,
                            "名前",
                            "Discordに表示する名前",
                            &mut self.draft.player_name,
                            48,
                        );
                        ui.add_space(10.0);
                        field(
                            ui,
                            "フレンドコード（任意）",
                            "例: ABCDEFGH",
                            &mut self.draft.friend_code,
                            48,
                        );
                        ui.add_space(18.0);

                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_sized(
                                    [96.0, 30.0],
                                    egui::Button::new(RichText::new("保存").strong())
                                        .fill(ACCENT)
                                        .corner_radius(4),
                                )
                                .clicked()
                                && self.save()
                            {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            }

                            let secondary_label =
                                if self.first_run { "スキップ" } else { "閉じる" };
                            if ui
                                .add_sized(
                                    [96.0, 30.0],
                                    egui::Button::new(secondary_label),
                                )
                                .clicked()
                            {
                                if self.first_run {
                                    if self.skip() {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                    }
                                } else {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                }
                            }
                        });

                        if let Some(message) = &self.feedback {
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(message).size(12.0).color(ERROR),
                                )
                                .wrap(),
                            );
                        }
                        if self.tray_icon.is_none() {
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new("通知領域アイコンを作成できませんでした")
                                        .size(12.0)
                                        .color(ERROR),
                                )
                                .wrap(),
                            );
                        }
                    });
            });
    }
}

fn field(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String, limit: usize) {
    ui.label(RichText::new(label).size(12.0).color(TEXT));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .char_limit(limit)
            .desired_width(f32::INFINITY)
            .margin(egui::vec2(8.0, 5.0)),
    );
}

fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

fn configfonts(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];
    let Some(bytes) = candidates.iter().find_map(|path| fs::read(path).ok()) else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "bluepresence-japanese".into(),
        Arc::new(FontData::from_owned(bytes)),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "bluepresence-japanese".into());
    ctx.set_fonts(fonts);
}

fn configstyle(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = BACKGROUND;
    style.visuals.extreme_bg_color = Color32::from_rgb(19, 21, 24);
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 52, 58);
    style.visuals.selection.bg_fill = ACCENT;
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    ctx.set_global_style(style);
}
