#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod apod;
use ferrite_design::prelude::*;
use gpui::{App, AppContext as _, ClickEvent, Context, Entity, IntoElement, KeyBinding, Render, Subscription, Task, Window, div, px, size};

struct Apogee {
    palette: Entity<CommandPalette>,
    _appearance: Subscription,
    entry: Option<apod::Entry>,
    picture: Option<(ferrite_design::dither::Picture, u32, u32)>,
    status: String,
    job: Option<Task<()>>,
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
        };
        let weak = cx.weak_entity();
        app.palette.update(cx, |palette, cx| {
            let mut commands = Vec::new();
            for scheme in SCHEMES {
                commands
                    .push(command(format!("Scheme: {}", scheme.name)).group("Theme").on_run(move |_, cx| theme::set_scheme(scheme, cx)));
            }
            commands.push(command("Toggle light / dark").group("Theme").on_run(|window, cx| {
                let appearance = if theme::appearance(cx) == Appearance::Light { Appearance::Dark } else { Appearance::Light };
                theme::set_appearance(appearance, window, cx);
            }));
            commands.push(command("Refresh APOD").group("Apogee").on_run(move |_, cx| {
                let _ = weak.update(cx, |this, cx| this.refresh(cx));
            }));
            palette.set_commands(commands, cx);
        });
        app.refresh(cx);
        app
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.job.is_some() {
            return;
        }
        self.status = "Fetching NASA APOD…".into();
        self.job = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let entry = apod::fetch()?;
                    let picture = apod::picture(&entry);
                    Ok::<_, String>((entry, picture))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.job = None;
                match result {
                    Ok((entry, picture)) => {
                        this.entry = Some(entry);
                        match picture {
                            Ok(picture) => {
                                this.picture = Some((
                                    ferrite_design::dither::Picture::new(picture.width, picture.height, picture.levels),
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

        let mut content: Vec<gpui::AnyElement> = vec![
            Button::new("refresh")
                .label("Refresh APOD")
                .primary()
                .disabled(self.job.is_some())
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .into_any_element(),
        ];
        if let Some(entry) = &self.entry {
            if let Some((picture, width, height)) = &self.picture {
                content.push(
                    panel(entry.title.clone())
                        .meta(entry.date.clone())
                        .child(dither(picture.clone()).w(px(*width as f32)).h(px(*height as f32)).ink(hsla(p.accent)))
                        .into_any_element(),
                );
            }
            content.push(
                panel(entry.title.clone())
                    .meta(format!("{} · {}", entry.date, entry.credit))
                    .child(div().p_4().body(text::BASE).child(entry.explanation.clone()))
                    .into_any_element(),
            );
            let url = entry.url.clone();
            content.push(
                Button::new("source")
                    .label(if entry.image { "Open NASA source" } else { "Watch video" })
                    .secondary()
                    .on_click(move |_, _, cx| cx.open_url(&url))
                    .into_any_element(),
            );
        }

        let menu =
            Button::new("commands").icon(Icon::Menu).ghost().small().tooltip("Commands · Ctrl+Shift+P").on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.palette.update(cx, |palette, cx| palette.toggle(window, cx))),
            );
        window_frame().child(power_on_in(
            "power",
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(hsla(p.bg))
                .text_color(hsla(p.fg))
                .body(text::BASE)
                .on_action(
                    cx.listener(|this, _: &TogglePalette, window, cx| this.palette.update(cx, |palette, cx| palette.toggle(window, cx))),
                )
                .child(self.palette.clone())
                .child(title_bar("Apogee").child(menu))
                .child(div().id("content").flex().flex_col().flex_1().min_h_0().overflow_y_scroll().gap_3().p_4().children(content))
                .child(status_bar().left_live(self.status.clone()).right(theme::scheme(cx).name.to_uppercase())),
        ))
    }
}
fn smoke() -> Result<(), String> {
    let entry = apod::fetch()?;
    if entry.image {
        let preview = apod::picture(&entry)?;
        println!("APOD: {} ({} x {})", entry.title, preview.width, preview.height);
    } else {
        println!("APOD video: {}", entry.title);
    }
    Ok(())
}

fn main() {
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
        let options = chrome::window_options("Apogee", size(px(1100.), px(800.)), cx);
        cx.open_window(options, |window, cx| {
            chrome::square_corners(window);
            chrome::power_off_on_close(window, cx);
            cx.new(|cx| Apogee::new(window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
}
