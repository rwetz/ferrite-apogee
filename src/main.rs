#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod apod;
mod probe;
mod worker;
use ferrite_design::prelude::*;
use gpui::{
    App, AppContext as _, ClickEvent, Context, Entity, IntoElement, KeyBinding, Render,
    Subscription, Task, Window, div, px, size,
};

struct Apogee {
    palette: Entity<CommandPalette>,
    _appearance: Subscription,
    entry: Option<apod::Entry>,
    picture: Option<(ferrite_design::dither::Picture, u32, u32)>,
    status: String,
    job: Option<Task<()>>,
    busy: bool,
}
impl Apogee {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            palette: cx.new(|cx| CommandPalette::new(window, cx)),
            _appearance: theme::follow_system(window),
            entry: None,
            picture: None,
            status: "Ready".into(),
            job: None,
            busy: false,
        };
        let weak = cx.weak_entity();
        app.palette.update(cx, |palette, cx| {
            let mut commands = Vec::new();
            for scheme in SCHEMES {
                commands.push(
                    command(format!("Scheme: {}", scheme.name))
                        .group("Theme")
                        .on_run(move |_, cx| theme::set_scheme(scheme, cx)),
                );
            }
            commands.push(
                command("Toggle light / dark")
                    .group("Theme")
                    .on_run(|window, cx| {
                        let appearance = if theme::appearance(cx) == Appearance::Light {
                            Appearance::Dark
                        } else {
                            Appearance::Light
                        };
                        theme::set_appearance(appearance, window, cx);
                    }),
            );
            commands.push(
                command("Refresh APOD")
                    .group("Apogee")
                    .on_run(move |_, cx| {
                        let _ = weak.update(cx, |this, cx| this.refresh(cx));
                    }),
            );
            palette.set_commands(commands, cx);
        });
        app.refresh(cx);
        app
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.status = "Fetching NASA APOD…".into();
        self.busy = true;
        self.job = Some(cx.spawn(async move |this, cx| {
            let result = worker::run(move || {
                let entry = apod::fetch()?;
                let picture = apod::picture(&entry);
                Ok::<_, String>((entry, picture))
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok((entry, picture)) => {
                        this.entry = Some(entry);
                        match picture {
                            Ok(picture) => {
                                this.picture = Some((
                                    ferrite_design::dither::Picture::new(
                                        picture.width,
                                        picture.height,
                                        picture.levels,
                                    ),
                                    picture.width,
                                    picture.height,
                                ));
                                this.status = "Ready".into();
                            }
                            Err(error) => {
                                this.picture = None;
                                this.status = error;
                            }
                        }
                    }
                    Err(error) => this.status = error,
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
impl Render for Apogee {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        if !self.busy && self.picture.is_some() {
            probe::record(format!(
                "APOD image: {}",
                self.entry
                    .as_ref()
                    .map(|e| e.title.as_str())
                    .unwrap_or_default()
            ));
        }

        let mut content: Vec<gpui::AnyElement> = vec![
            Button::new("refresh")
                .label("Refresh APOD")
                .primary()
                .disabled(self.busy)
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .into_any_element(),
        ];
        content.push(
            div()
                .p_3()
                .bg(hsla(p.surface))
                .text_color(hsla(p.fg_dim))
                .child(self.status.clone())
                .into_any_element(),
        );
        if let Some(entry) = &self.entry {
            if let Some((picture, width, height)) = &self.picture {
                content.push(
                    panel("Astronomy picture")
                        .meta(entry.date.clone())
                        .child({
                            let picture = picture.clone();
                            let aspect = *width as f32 / *height as f32;
                            responsive("picture-room", move |room, _, cx| {
                                let p = palette(cx);
                                let height =
                                    f32::from(room.height).min(f32::from(room.width) / aspect);
                                div()
                                    .size_full()
                                    .flex()
                                    .justify_center()
                                    .items_center()
                                    .child(
                                        dither(picture.clone())
                                            .w(px(height * aspect))
                                            .h(px(height))
                                            .ink(hsla(p.accent))
                                            .paper(hsla(p.bg)),
                                    )
                                    .into_any_element()
                            })
                            .w_full()
                            .h(px(480.))
                        })
                        .into_any_element(),
                );
            }
            content.push(
                panel("About today's picture")
                    .meta(entry.date.clone())
                    .child(
                        div()
                            .body(text::BASE)
                            .text_color(hsla(p.accent))
                            .child(entry.title.clone()),
                    )
                    .child(div().text_color(hsla(p.fg_dim)).child(entry.credit.clone()))
                    .child(
                        div().w_full().flex().justify_center().child(
                            div()
                                .w_full()
                                .max_w(px(960.))
                                .p_4()
                                .body(text::BASE)
                                .child(entry.explanation.clone()),
                        ),
                    )
                    .into_any_element(),
            );
            let url = entry.url.clone();
            content.push(
                Button::new("source")
                    .label(if entry.image {
                        "Open NASA source"
                    } else {
                        "Watch video"
                    })
                    .secondary()
                    .on_click(move |_, _, cx| cx.open_url(&url))
                    .into_any_element(),
            );
        }

        let menu = Button::new("commands")
            .icon(Icon::Menu)
            .ghost()
            .small()
            .tooltip("Commands · Ctrl+Shift+P")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.palette
                    .update(cx, |palette, cx| palette.toggle(window, cx))
            }));
        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .on_action(cx.listener(|this, _: &TogglePalette, window, cx| {
                    this.palette
                        .update(cx, |palette, cx| palette.toggle(window, cx))
                }))
                .child(self.palette.clone())
                .child(title_bar("Apogee").child(menu))
                .child(
                    div()
                        .id("content")
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .gap_3()
                        .p_4()
                        .children(content),
                )
                .child(
                    status_bar()
                        .left_live(self.status.clone())
                        .right(theme::scheme(cx).name.to_uppercase()),
                ),
        ))
    }
}
fn smoke() -> Result<(), String> {
    let entry = apod::fetch()?;
    if entry.image {
        let preview = apod::picture(&entry)?;
        println!(
            "APOD: {} ({} x {})",
            entry.title, preview.width, preview.height
        );
    } else {
        println!("APOD video: {}", entry.title);
    }
    Ok(())
}

fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args().any(|arg| arg == "--smoke") {
        if let Err(error) = smoke() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    gpui_platform::application().run(|cx: &mut App| {
        ferrite_design::init(Appearance::Dark, cx);
        theme::apply_env(cx);
        cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
        let options =
            chrome::remembered_window_options("apogee", "Apogee", size(px(1100.), px(800.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::remember_window("apogee", window, cx);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Apogee::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
