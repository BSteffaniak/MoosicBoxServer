#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, LazyLock, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use async_trait::async_trait;
use canvas::CanvasAction;
use eframe::egui::{self, Color32, CursorIcon, Response, Ui, Widget};
use flume::{Receiver, Sender};
use gigachad_actions::{ActionTrigger, ActionType, ElementTarget, StyleAction};
use gigachad_renderer::canvas::CanvasUpdate;
use gigachad_renderer::viewport::immediate::{Pos, Viewport, ViewportListener};
pub use gigachad_renderer::*;
use gigachad_router::Router;
use gigachad_transformer::{
    calc::Calc,
    models::{
        AlignItems, Cursor, JustifyContent, LayoutDirection, LayoutOverflow, LayoutPosition,
        Position, Route, SwapTarget, Visibility,
    },
    Container, Element, Input, TableIter,
};
use itertools::Itertools;

#[cfg(feature = "debug")]
static DEBUG: LazyLock<RwLock<bool>> = LazyLock::new(|| {
    RwLock::new(
        std::env::var("DEBUG_RENDERER")
            .is_ok_and(|x| ["1", "true"].contains(&x.to_lowercase().as_str())),
    )
});

#[derive(Clone)]
pub struct EguiRenderer {
    width: Option<f32>,
    height: Option<f32>,
    x: Option<i32>,
    y: Option<i32>,
    app: EguiApp,
    receiver: Receiver<String>,
}

impl EguiRenderer {
    #[must_use]
    pub fn new(
        router: Router,
        request_action: Sender<String>,
        on_resize: Sender<(f32, f32)>,
    ) -> Self {
        let (tx, rx) = flume::unbounded();
        let (event_tx, event_rx) = flume::unbounded();
        Self {
            width: None,
            height: None,
            x: None,
            y: None,
            app: EguiApp::new(router, tx, event_tx, event_rx, request_action, on_resize),
            receiver: rx,
        }
    }

    #[must_use]
    pub async fn wait_for_navigation(&self) -> Option<String> {
        self.receiver.recv_async().await.ok()
    }
}

pub struct EguiRenderRunner {
    width: f32,
    height: f32,
    x: Option<i32>,
    y: Option<i32>,
    app: EguiApp,
}

#[cfg_attr(feature = "profiling", profiling::all_functions)]
impl RenderRunner for EguiRenderRunner {
    /// # Errors
    ///
    /// Will error if egui fails to run the event loop.
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut viewport =
            egui::ViewportBuilder::default().with_inner_size([self.width, self.height]);

        #[allow(clippy::cast_precision_loss)]
        if let (Some(x), Some(y)) = (self.x, self.y) {
            viewport = viewport.with_position((x as f32, y as f32));
        }

        gigachad_transformer::calc::set_scrollbar_size(0);

        #[cfg(feature = "wgpu")]
        let renderer = eframe::Renderer::Wgpu;
        #[cfg(not(feature = "wgpu"))]
        let renderer = eframe::Renderer::Glow;

        let options = eframe::NativeOptions {
            viewport,
            centered: true,
            renderer,
            ..Default::default()
        };

        #[cfg(feature = "profiling-tracing")]
        tracing_subscriber::fmt::init();
        #[cfg(feature = "profiling-puffin")]
        start_puffin_server();

        log::debug!("run: starting");
        if let Err(e) = eframe::run_native(
            "MoosicBox",
            options,
            Box::new(|cc| {
                egui_extras::install_image_loaders(&cc.egui_ctx);
                let app = self.app.clone();
                *app.ctx.write().unwrap() = Some(cc.egui_ctx.clone());
                Ok(Box::new(app))
            }),
        ) {
            log::error!("run: eframe error: {e:?}");
        }
        log::debug!("run: finished");

        Ok(())
    }
}

#[async_trait]
impl Renderer for EguiRenderer {
    /// # Panics
    ///
    /// Will panic if elements `Mutex` is poisoned.
    ///
    /// # Errors
    ///
    /// Will error if egui app fails to start
    async fn init(
        &mut self,
        width: f32,
        height: f32,
        x: Option<i32>,
        y: Option<i32>,
        background: Option<Color>,
    ) -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
        self.width = Some(width);
        self.height = Some(height);
        self.x = x;
        self.y = y;
        self.app.background = background.map(Into::into);

        log::debug!("start: spawning listen thread");
        moosicbox_task::spawn("renderer_egui::start: listen", {
            let app = self.app.clone();
            async move {
                log::debug!("start: listening");
                app.listen().await;
                Ok::<_, Box<dyn std::error::Error + Send + 'static>>(())
            }
        });

        Ok(())
    }

    /// # Errors
    ///
    /// Will error if egui fails to run the event loop.
    async fn to_runner(&self) -> Result<Box<dyn RenderRunner>, Box<dyn std::error::Error + Send>> {
        Ok(Box::new(EguiRenderRunner {
            width: self.width.unwrap(),
            height: self.height.unwrap(),
            x: self.x,
            y: self.y,
            app: self.app.clone(),
        }))
    }

    /// # Errors
    ///
    /// Will error if egui app fails to emit the event.
    async fn emit_event(
        &self,
        event_name: String,
        event_value: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
        log::trace!("emit_event: event_name={event_name} event_value={event_value:?}");

        let app = self.app.clone();

        moosicbox_task::spawn_blocking("handle_event", move || {
            app.handle_event(&event_name, event_value.as_deref());
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + 'static>)?;

        Ok(())
    }

    /// # Errors
    ///
    /// Will error if egui fails to render the view.
    ///
    /// # Panics
    ///
    /// Will panic if elements `Mutex` is poisoned.
    async fn render(&self, view: View) -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
        let app = self.app.clone();
        let width = self.width;
        let height = self.height;

        moosicbox_task::spawn_blocking("egui render", move || {
            moosicbox_logging::debug_or_trace!(
                ("render: start"),
                ("render: start {:?}", view.immediate)
            );
            let mut elements = view.immediate;

            elements.calculated_width = app.width.read().unwrap().or(width);
            elements.calculated_height = app.height.read().unwrap().or(height);
            elements.calc();
            *app.container.write().unwrap() = elements;
            app.images.write().unwrap().clear();
            app.backgrounds.write().unwrap().clear();
            app.viewport_listeners.write().unwrap().clear();
            app.route_requests.write().unwrap().clear();
            app.checkboxes.write().unwrap().clear();

            log::debug!("render: finished");
            if let Some(ctx) = &*app.ctx.read().unwrap() {
                ctx.request_repaint();
            }

            Ok::<_, Box<dyn std::error::Error + Send + 'static>>(())
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + 'static>)??;

        Ok(())
    }

    /// # Errors
    ///
    /// Will error if egui fails to render the partial view.
    ///
    /// # Panics
    ///
    /// Will panic if elements `Mutex` is poisoned.
    async fn render_partial(
        &self,
        view: PartialView,
    ) -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
        let app = self.app.clone();

        moosicbox_task::spawn_blocking("egui render_partial", move || {
            moosicbox_logging::debug_or_trace!(
                ("render_partial: start"),
                ("render_partial: start {:?}", view)
            );

            let mut page = app.container.write().unwrap();
            let ids = view
                .container
                .children
                .as_slice()
                .iter()
                .map(|x| x.id)
                .collect::<Vec<_>>();
            if let Some(removed) =
                page.replace_str_id_with_elements(view.container.children, &view.target, true)
            {
                let mut visibilities = app.visibilities.write().unwrap();
                if let Some(visibility) = visibilities.remove(&removed.id) {
                    for id in &ids {
                        visibilities.insert(*id, visibility);
                    }
                }
                drop(visibilities);
                let mut backgrounds = app.backgrounds.write().unwrap();
                if let Some(background) = backgrounds.remove(&removed.id) {
                    for id in &ids {
                        backgrounds.insert(*id, background.clone());
                    }
                }
                drop(backgrounds);
                drop(page);
                if let Some(ctx) = &*app.ctx.read().unwrap() {
                    ctx.request_repaint();
                }
            } else {
                log::warn!("Unable to find element with id {}", view.target);
            }

            Ok::<_, Box<dyn std::error::Error + Send + 'static>>(())
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + 'static>)??;

        Ok(())
    }

    /// # Errors
    ///
    /// Will error if egui fails to render the canvas update.
    ///
    /// # Panics
    ///
    /// Will panic if elements `Mutex` is poisoned.
    async fn render_canvas(
        &self,
        mut update: CanvasUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + 'static>> {
        let app = self.app.clone();

        moosicbox_task::spawn_blocking("egui render_canvas", move || {
            log::trace!("render_canvas: start");

            let mut binding = app.canvas_actions.write().unwrap();

            let actions = binding
                .entry(update.target)
                .or_insert_with(|| Vec::with_capacity(update.canvas_actions.len()));

            actions.append(&mut update.canvas_actions);

            compact_canvas_actions(actions);

            drop(binding);

            if let Some(ctx) = &*app.ctx.read().unwrap() {
                ctx.request_repaint();
            }

            log::trace!("render_canvas: end");

            Ok::<_, Box<dyn std::error::Error + Send + 'static>>(())
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + 'static>)??;

        Ok(())
    }

    fn container(&self) -> RwLockReadGuard<Container> {
        self.app.container.read().unwrap()
    }

    fn container_mut(&self) -> RwLockWriteGuard<Container> {
        self.app.container.write().unwrap()
    }
}

fn compact_canvas_actions(actions: &mut Vec<CanvasAction>) {
    let len = actions.len();
    for i in 0..len {
        let i = len - 1 - i;
        if matches!(actions[i], CanvasAction::Clear) {
            actions.drain(..=i);
            return;
        }
    }
}

#[derive(Debug)]
enum AppEvent {
    LoadImage { source: String },
    ProcessRoute { route: Route, container_id: usize },
}

#[derive(Clone)]
enum AppImage {
    Loading,
    Bytes(Arc<[u8]>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleTrigger {
    UiEvent,
    CustomEvent,
}

#[derive(Debug, Clone)]
struct StyleOverride<T> {
    trigger: StyleTrigger,
    value: T,
}

struct RenderContext<'a> {
    container: &'a Container,
    viewport_listeners: &'a mut HashMap<usize, ViewportListener>,
    images: &'a mut HashMap<String, AppImage>,
    canvas_actions: &'a mut HashMap<String, Vec<CanvasAction>>,
    route_requests: &'a mut Vec<usize>,
    visibilities: &'a mut HashMap<usize, Visibility>,
    displays: &'a mut HashMap<usize, bool>,
    backgrounds: &'a mut HashMap<usize, Vec<StyleOverride<Option<Color>>>>,
    checkboxes: &'a mut HashMap<egui::Id, bool>,
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
struct EguiApp {
    ctx: Arc<RwLock<Option<egui::Context>>>,
    width: Arc<RwLock<Option<f32>>>,
    height: Arc<RwLock<Option<f32>>>,
    container: Arc<RwLock<Container>>,
    sender: Sender<String>,
    event: Sender<AppEvent>,
    event_receiver: Receiver<AppEvent>,
    viewport_listeners: Arc<RwLock<HashMap<usize, ViewportListener>>>,
    images: Arc<RwLock<HashMap<String, AppImage>>>,
    canvas_actions: Arc<RwLock<HashMap<String, Vec<CanvasAction>>>>,
    route_requests: Arc<RwLock<Vec<usize>>>,
    visibilities: Arc<RwLock<HashMap<usize, Visibility>>>,
    displays: Arc<RwLock<HashMap<usize, bool>>>,
    backgrounds: Arc<RwLock<HashMap<usize, Vec<StyleOverride<Option<Color>>>>>>,
    checkboxes: Arc<RwLock<HashMap<egui::Id, bool>>>,
    router: Router,
    background: Option<Color32>,
    request_action: Sender<String>,
    on_resize: Sender<(f32, f32)>,
    side_effects: Arc<Mutex<VecDeque<Handler>>>,
    event_handlers: Arc<RwLock<Vec<(String, EventHandler)>>>,
}

type Handler = Box<dyn Fn(&mut RenderContext) -> bool + Send + Sync>;
type EventHandler = Box<dyn Fn(&mut RenderContext, Option<&str>) + Send + Sync>;

impl EguiApp {
    fn new(
        router: Router,
        sender: Sender<String>,
        event: Sender<AppEvent>,
        event_receiver: Receiver<AppEvent>,
        request_action: Sender<String>,
        on_resize: Sender<(f32, f32)>,
    ) -> Self {
        Self {
            ctx: Arc::new(RwLock::new(None)),
            width: Arc::new(RwLock::new(None)),
            height: Arc::new(RwLock::new(None)),
            container: Arc::new(RwLock::new(Container::default())),
            sender,
            event,
            event_receiver,
            viewport_listeners: Arc::new(RwLock::new(HashMap::new())),
            images: Arc::new(RwLock::new(HashMap::new())),
            canvas_actions: Arc::new(RwLock::new(HashMap::new())),
            route_requests: Arc::new(RwLock::new(vec![])),
            visibilities: Arc::new(RwLock::new(HashMap::new())),
            displays: Arc::new(RwLock::new(HashMap::new())),
            backgrounds: Arc::new(RwLock::new(HashMap::new())),
            checkboxes: Arc::new(RwLock::new(HashMap::new())),
            router,
            background: None,
            request_action,
            on_resize,
            side_effects: Arc::new(Mutex::new(VecDeque::new())),
            event_handlers: Arc::new(RwLock::new(vec![])),
        }
    }

    /// # Errors
    ///
    /// Will error if egui fails to emit the event.
    fn handle_event(&self, event_name: &str, event_value: Option<&str>) {
        log::debug!("handle_event: event_name={event_name} event_value={event_value:?}");

        let container = self.container.write().unwrap();
        let mut viewport_listeners = self.viewport_listeners.write().unwrap();
        let mut images = self.images.write().unwrap();
        let mut canvas_actions = self.canvas_actions.write().unwrap();
        let mut route_requests = self.route_requests.write().unwrap();
        let mut visibilities = self.visibilities.write().unwrap();
        let mut displays = self.displays.write().unwrap();
        let mut backgrounds = self.backgrounds.write().unwrap();
        let mut checkboxes = self.checkboxes.write().unwrap();

        let mut render_context = RenderContext {
            container: &container,
            viewport_listeners: &mut viewport_listeners,
            images: &mut images,
            canvas_actions: &mut canvas_actions,
            route_requests: &mut route_requests,
            visibilities: &mut visibilities,
            backgrounds: &mut backgrounds,
            displays: &mut displays,
            checkboxes: &mut checkboxes,
        };

        let ctx = self.ctx.read().unwrap().clone();
        let binding = self.event_handlers.read().unwrap();
        for handler in binding.iter().filter_map(|(name, handler)| {
            if name == event_name {
                Some(handler)
            } else {
                None
            }
        }) {
            handler(&mut render_context, event_value);

            if let Some(ctx) = &ctx {
                ctx.request_repaint();
            };
        }

        drop(container);
        drop(binding);
    }

    #[allow(clippy::too_many_lines)]
    async fn listen(&self) {
        while let Ok(event) = self.event_receiver.recv_async().await {
            log::trace!("received event {event:?}");
            match event {
                AppEvent::LoadImage { source } => {
                    let images = self.images.clone();
                    let ctx = self.ctx.clone();
                    if let Some(bytes) = moosicbox_app_native_image::get_image(&source) {
                        log::trace!("loading image {source}");
                        images
                            .write()
                            .unwrap()
                            .insert(source, AppImage::Bytes(bytes.to_vec().into()));

                        if let Some(ctx) = &*ctx.read().unwrap() {
                            ctx.request_repaint();
                        }
                    } else {
                        moosicbox_task::spawn("renderer: load_image", async move {
                            static CLIENT: LazyLock<reqwest::Client> =
                                LazyLock::new(reqwest::Client::new);

                            log::trace!("loading image {source}");
                            match CLIENT.get(&source).send().await {
                                Ok(response) => {
                                    if !response.status().is_success() {
                                        log::error!(
                                            "Failed to load image: {}",
                                            response.text().await.unwrap_or_else(|e| {
                                                format!("(failed to get response text: {e:?})")
                                            })
                                        );
                                        return;
                                    }

                                    match response.bytes().await {
                                        Ok(bytes) => {
                                            let bytes = bytes.to_vec().into();

                                            let mut binding = images.write().unwrap();
                                            binding.insert(source, AppImage::Bytes(bytes));
                                            drop(binding);

                                            if let Some(ctx) = &*ctx.read().unwrap() {
                                                ctx.request_repaint();
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to fetch image ({source}): {e:?}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to fetch image ({source}): {e:?}");
                                }
                            }
                        });
                    }
                }
                AppEvent::ProcessRoute {
                    route,
                    container_id,
                } => {
                    let router = self.router.clone();
                    let container = self.container.clone();
                    let ctx = self.ctx.clone();
                    moosicbox_task::spawn("renderer: ProcessRoute", async move {
                        match route {
                            Route::Get {
                                route,
                                trigger,
                                swap,
                            }
                            | Route::Post {
                                route,
                                trigger,
                                swap,
                            } => {
                                if trigger.as_deref() == Some("load") {
                                    match router.navigate(&route).await {
                                        Ok(result) => {
                                            let Some(ctx) = ctx.read().unwrap().clone() else {
                                                moosicbox_assert::die_or_panic!(
                                                    "Context was not set"
                                                )
                                            };
                                            Self::swap_elements(
                                                &swap,
                                                &ctx,
                                                &container,
                                                container_id,
                                                result.immediate,
                                            );
                                            if let Some(future) = result.future {
                                                let result = future.await;
                                                Self::swap_elements(
                                                    &swap,
                                                    &ctx,
                                                    &container,
                                                    container_id,
                                                    result,
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to process route ({route}): {e:?}");
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn swap_elements(
        swap: &SwapTarget,
        ctx: &egui::Context,
        container: &RwLock<Container>,
        container_id: usize,
        result: Container,
    ) {
        log::debug!(
            "ProcessRoute: replacing container_id={container_id} with {} elements",
            result.children.len()
        );
        let mut page = container.write().unwrap();
        match swap {
            SwapTarget::This => {
                if page.replace_id_with_elements(result.children, container_id, true) {
                    drop(page);
                    ctx.request_repaint();
                } else {
                    log::warn!("Unable to find element with id {container_id}");
                }
            }
            SwapTarget::Children => {
                if page.replace_id_children_with_elements(result.children, container_id, true) {
                    drop(page);
                    ctx.request_repaint();
                } else {
                    log::warn!("Unable to find element with id {container_id}");
                }
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn update_frame_size(&self, width: f32, height: f32) {
        *self.viewport_listeners.write().unwrap() = HashMap::new();

        log::debug!(
            "calc: frame size changed from ({:?}, {:?}) -> ({width}, {height})",
            self.width.read().unwrap(),
            self.height.read().unwrap()
        );

        {
            let mut container = self.container.write().unwrap();
            container.calculated_width.replace(width);
            container.calculated_height.replace(height);
            container.calc();
        }

        self.width.write().unwrap().replace(width);
        self.height.write().unwrap().replace(height);
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn check_frame_resize(&self, ctx: &egui::Context) {
        ctx.input(move |i| {
            let width = i.screen_rect.width();
            let height = i.screen_rect.height();
            let current_width = *self.width.read().unwrap();
            let current_height = *self.height.read().unwrap();
            if current_width.is_none_or(|x| (x - width).abs() >= 0.01)
                || current_height.is_none_or(|x| (x - height).abs() >= 0.01)
            {
                self.update_frame_size(width, height);
                if let Err(e) = self.on_resize.send((width, height)) {
                    moosicbox_assert::die_or_error!(
                        "Failed to send on_resize message: {width}, {height}: {e:?}"
                    );
                }
            }
        });
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_scroll_container(
        rect: egui::Rect,
        pos_x: f32,
        pos_y: f32,
        element: &Container,
        parent: Option<&Viewport>,
    ) -> Viewport {
        let viewport = Viewport {
            parent: parent.cloned().map(Box::new),
            pos: Pos {
                x: pos_x,
                y: pos_y,
                w: element.calculated_width.unwrap(),
                h: element.calculated_height.unwrap(),
            },
            viewport: Pos {
                x: rect.min.x,
                y: rect.min.y,
                w: element.calculated_width.unwrap(),
                h: element.calculated_height.unwrap(),
            },
        };

        log::trace!(
            "get_scroll_container: ({}, {})",
            viewport.pos.x,
            viewport.pos.y
        );

        viewport
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_horizontal_borders(
        ui: &mut Ui,
        container: &Container,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        ui.horizontal(|ui| {
            if let Some((color, size)) = container.calculated_border_left {
                egui::Frame::none().fill(color.into()).show(ui, |ui| {
                    ui.set_width(size);
                    ui.set_height(container.calculated_height.unwrap_or(0.0));
                });
            }

            let response = add_contents(ui);

            if let Some((color, size)) = container.calculated_border_right {
                egui::Frame::none().fill(color.into()).show(ui, |ui| {
                    ui.set_width(size);
                    ui.set_height(container.calculated_height.unwrap_or(0.0));
                });
            }

            response
        })
        .inner
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_vertical_borders(
        ui: &mut Ui,
        container: &Container,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        ui.vertical(|ui| {
            if let Some((color, size)) = container.calculated_border_top {
                egui::Frame::none().fill(color.into()).show(ui, |ui| {
                    ui.set_width(container.calculated_width.unwrap_or(0.0));
                    ui.set_height(size);
                });
            }

            let response = add_contents(ui);

            if let Some((color, size)) = container.calculated_border_bottom {
                egui::Frame::none().fill(color.into()).show(ui, |ui| {
                    ui.set_width(container.calculated_width.unwrap_or(0.0));
                    ui.set_height(size);
                });
            }

            response
        })
        .inner
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_borders(
        ui: &mut Ui,
        container: &Container,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        if container.calculated_border_left.is_some() || container.calculated_border_right.is_some()
        {
            Self::render_horizontal_borders(ui, container, |ui| {
                if container.calculated_border_top.is_some()
                    || container.calculated_border_bottom.is_some()
                {
                    Self::render_vertical_borders(ui, container, add_contents)
                } else {
                    add_contents(ui)
                }
            })
        } else if container.calculated_border_top.is_some()
            || container.calculated_border_bottom.is_some()
        {
            Self::render_vertical_borders(ui, container, add_contents)
        } else {
            add_contents(ui)
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn container_hidden(render_context: &mut RenderContext, container: &Container) -> bool {
        if container.visibility == Some(Visibility::Hidden) {
            render_context
                .visibilities
                .get(&container.id)
                .copied()
                .unwrap_or(Visibility::Hidden)
                == Visibility::Hidden
        } else {
            false
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    fn handle_scroll_child_out_of_view(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        container: &Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
    ) -> bool {
        if let Some(rect) = rect {
            let render_rect = Self::get_render_rect(ui, container, relative_container);
            let width = render_rect.width();
            let height = render_rect.height();
            let (offset_x, offset_y) =
                viewport.map_or((0.0, 0.0), |viewport| (viewport.pos.x, viewport.pos.y));

            if render_rect.min.x + width - offset_x < -1.0
                || render_rect.min.y + height - offset_y < -1.0
                || render_rect.min.x - offset_x >= rect.width() + 1.0
                || render_rect.min.y - offset_y >= rect.height() + 1.0
            {
                log::trace!(
                    "render_container: skipping ({}, {}, {width}, {height})",
                    render_rect.min.x,
                    render_rect.min.y
                );
                self.handle_container_side_effects(
                    render_context,
                    ctx,
                    Some(ui),
                    container,
                    viewport,
                    Some(rect),
                    None,
                    true,
                );
                ui.allocate_space(egui::vec2(width, height));
                return true;
            }
            log::trace!(
                "render_container: showing ({}, {}, {width}, {height})",
                render_rect.min.x,
                render_rect.min.y
            );
        }

        false
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    fn render_container(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        container: &Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
    ) -> Option<Response> {
        if container.debug == Some(true) {
            if let Some(element) = render_context.container.find_element_by_id(container.id) {
                log::info!("render_container: DEBUG {element}");
            }
        }
        if container.is_hidden() || Self::container_hidden(render_context, container) {
            log::trace!("render_container: container is hidden. skipping render");
            self.handle_container_side_effects(
                render_context,
                ctx,
                Some(ui),
                container,
                viewport,
                rect,
                None,
                true,
            );
            return None;
        }

        if let Some(opacity) = container.calculated_opacity {
            ui.set_opacity(opacity);
        }

        Some(Self::render_borders(ui, container, |ui| {
            let (
                render_context,
                response,
            ) = egui::Frame::none()
                .inner_margin(egui::Margin {
                    left:
                        container.internal_margin_left.unwrap_or(0.0)
                        + container.calculated_margin_left.unwrap_or(0.0),
                    right:
                        container.internal_margin_right.unwrap_or(0.0)
                        + container.calculated_margin_right.unwrap_or(0.0),
                    top:
                        container.internal_margin_top.unwrap_or(0.0)
                        + container.calculated_margin_top.unwrap_or(0.0),
                    bottom:
                        container.internal_margin_bottom.unwrap_or(0.0)
                        + container.calculated_margin_bottom.unwrap_or(0.0),
                })
                .show(ui, {
                    move |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            egui::Frame::none().show(ui, {
                                move |ui| {
                                    let cursor = ui.cursor();
                                    let (pos_x, pos_y) = (cursor.left(), cursor.top());
                                    match (container.overflow_x, container.overflow_y) {
                                        (
                                            LayoutOverflow::Auto,
                                            LayoutOverflow::Auto,
                                        ) => {
                                            egui::ScrollArea::both()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            true,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (
                                            LayoutOverflow::Scroll,
                                            LayoutOverflow::Scroll,
                                        ) => {
                                            egui::ScrollArea::both()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            true,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (
                                            LayoutOverflow::Auto,
                                            LayoutOverflow::Scroll,
                                        ) => {
                                            egui::ScrollArea::vertical()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let cursor = ui.cursor();
                                                        let (pos_x, pos_y) = (cursor.left(), cursor.top());
                                                        let (render_context, response) = egui::ScrollArea::horizontal()
                                                            .scroll_bar_visibility(
                                                                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                            )
                                                            .show_viewport(ui, {
                                                                move |ui, rect| {
                                                                    let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                                    let viewport = Some(&viewport);
                                                                    let response = self.render_container_contents(
                                                                        render_context,
                                                                        ctx,
                                                                        ui,
                                                                        container,
                                                                        viewport,
                                                                        Some(rect),
                                                                        relative_container,
                                                                        true,
                                                                    );

                                                                    (render_context, response)
                                                                }
                                                            }).inner;

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (
                                            LayoutOverflow::Scroll,
                                            LayoutOverflow::Auto,
                                        ) => {
                                            egui::ScrollArea::vertical()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let cursor = ui.cursor();
                                                        let (pos_x, pos_y) = (cursor.left(), cursor.top());
                                                        let (render_context, response) = egui::ScrollArea::horizontal()
                                                            .scroll_bar_visibility(
                                                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                                            )
                                                            .show_viewport(ui, {
                                                                move |ui, rect| {
                                                                    let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                                    let viewport = Some(&viewport);
                                                                    let response = self.render_container_contents(
                                                                        render_context,
                                                                        ctx,
                                                                        ui,
                                                                        container,
                                                                        viewport,
                                                                        Some(rect),
                                                                        relative_container,
                                                                        true,
                                                                    );

                                                                    (render_context, response)
                                                                }
                                                            }).inner;

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (LayoutOverflow::Auto, _) => {
                                            egui::ScrollArea::horizontal()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            false,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (LayoutOverflow::Scroll, _) => {
                                            egui::ScrollArea::horizontal()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            false,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (_, LayoutOverflow::Auto) => {
                                            egui::ScrollArea::vertical()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            true,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (_, LayoutOverflow::Scroll) => {
                                            egui::ScrollArea::vertical()
                                                .scroll_bar_visibility(
                                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                                )
                                                .show_viewport(ui, {
                                                    move |ui, rect| {
                                                        let viewport = Self::get_scroll_container(rect, pos_x, pos_y, container, viewport);
                                                        let viewport = Some(&viewport);
                                                        let response = self.render_container_contents(
                                                            render_context,
                                                            ctx,
                                                            ui,
                                                            container,
                                                            viewport,
                                                            Some(rect),
                                                            relative_container,
                                                            true,
                                                        );

                                                        (render_context, response)
                                                    }
                                                }).inner
                                        }
                                        (_, _) => {
                                            let response = self.render_container_contents(
                                                render_context,
                                                ctx,
                                                ui,
                                                container,
                                                viewport,
                                                rect,
                                                relative_container,
                                                false,
                                            );

                                            (render_context, response)
                                        }
                                    }
                                }
                            })
                        }).inner.inner
                    }
                }).inner;

            ui.set_opacity(1.0);

            if !Self::container_hidden(render_context, container) {
                self.handle_container_side_effects(
                    render_context,
                    ctx,
                    Some(ui),
                    container,
                    viewport,
                    rect,
                    Some(&response),
                    false,
                );
            }

            response
        }))
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_render_rect(
        ui: &Ui,
        container: &Container,
        relative_container: Option<(egui::Rect, &Container)>,
    ) -> egui::Rect {
        if container.position == Some(Position::Absolute) {
            if let Some((relative_rect, ..)) = relative_container {
                return relative_rect
                    .with_min_x(relative_rect.min.x + container.calculated_x.unwrap())
                    .with_min_y(relative_rect.min.y + container.calculated_y.unwrap())
                    .with_max_x(
                        relative_rect.min.x
                            + container.calculated_x.unwrap()
                            + container.bounding_calculated_width().unwrap(),
                    )
                    .with_max_y(
                        relative_rect.min.y
                            + container.calculated_y.unwrap()
                            + container.bounding_calculated_height().unwrap(),
                    );
            }
        }

        egui::Rect::from_min_size(
            ui.cursor().left_top(),
            egui::vec2(
                container.calculated_width.unwrap(),
                container.calculated_height.unwrap(),
            ),
        )
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_position<'a>(
        ui: &mut Ui,
        container: &'a Container,
        mut relative_container: Option<(egui::Rect, &'a Container)>,
        inner: impl FnOnce(&mut Ui, Option<(egui::Rect, &'a Container)>) -> Response,
    ) -> Response {
        match container.position {
            Some(Position::Relative) => {
                let pos = ui.cursor().left_top();
                let size = egui::vec2(
                    container.calculated_width.unwrap(),
                    container.calculated_height.unwrap(),
                );
                relative_container = Some((egui::Rect::from_min_size(pos, size), container));
            }
            Some(Position::Absolute) => {
                let abs_rect = Self::get_render_rect(ui, container, relative_container);

                return ui.put(abs_rect, |ui: &mut egui::Ui| inner(ui, relative_container));
            }
            Some(Position::Static) | None => {}
        }

        inner(ui, relative_container)
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    fn render_direction<'a>(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        container: &'a Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &'a Container)>,
        vscroll: bool,
    ) -> Response {
        for element in container.children.iter().filter(|x| x.is_hidden()) {
            self.handle_container_side_effects(
                render_context,
                ctx,
                None,
                element,
                viewport,
                rect,
                None,
                true,
            );
        }

        match container.direction {
            LayoutDirection::Row => {
                let rows = container
                    .children
                    .iter()
                    .filter_map(|x| x.calculated_position.as_ref().map(|y| (x, y)))
                    .filter_map({
                        |(x, y)| match y {
                            LayoutPosition::Wrap { row, .. } => Some((*row, x)),
                            LayoutPosition::Default => None,
                        }
                    })
                    .chunk_by(|(row, _element)| *row);

                let mut rows = rows
                    .into_iter()
                    .map(|(_row, y)| y.into_iter().map(|(_, element)| element).collect_vec())
                    .peekable();

                if rows.peek().is_some() {
                    ui.vertical(move |ui| {
                        for row in rows {
                            let render_context = &mut *render_context;
                            ui.horizontal(move |ui| {
                                self.render_elements_ref(
                                    render_context,
                                    ctx,
                                    ui,
                                    &row,
                                    viewport,
                                    rect,
                                    relative_container,
                                    !vscroll && rect.is_some(),
                                );
                            });
                        }
                    })
                    .response
                } else {
                    ui.horizontal(move |ui| {
                        self.render_elements(
                            render_context,
                            ctx,
                            ui,
                            &container.children,
                            viewport,
                            rect,
                            relative_container,
                            !vscroll && rect.is_some(),
                        );
                    })
                    .response
                }
            }
            LayoutDirection::Column => {
                let cols = container
                    .children
                    .iter()
                    .filter_map(|x| x.calculated_position.as_ref().map(|y| (x, y)))
                    .filter_map(|(x, y)| match y {
                        LayoutPosition::Wrap { col, .. } => Some((*col, x)),
                        LayoutPosition::Default => None,
                    })
                    .chunk_by(|(col, _element)| *col);

                let mut cols = cols
                    .into_iter()
                    .map(|(_row, y)| y.into_iter().map(|(_, element)| element).collect_vec())
                    .peekable();

                if cols.peek().is_some() {
                    ui.horizontal(move |ui| {
                        for col in cols {
                            let render_context = &mut *render_context;
                            ui.vertical(move |ui| {
                                self.render_elements_ref(
                                    render_context,
                                    ctx,
                                    ui,
                                    &col,
                                    viewport,
                                    rect,
                                    relative_container,
                                    !vscroll && rect.is_some(),
                                );
                            });
                        }
                    })
                    .response
                } else {
                    ui.vertical(move |ui| {
                        self.render_elements(
                            render_context,
                            ctx,
                            ui,
                            &container.children,
                            viewport,
                            rect,
                            relative_container,
                            !vscroll && rect.is_some(),
                        );
                    })
                    .response
                }
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_layout<'a>(
        ui: &mut Ui,
        container: &'a Container,
        relative_container: Option<(egui::Rect, &'a Container)>,
        inner: impl FnOnce(&mut Ui, Option<(egui::Rect, &'a Container)>) -> Response,
    ) -> Response {
        if matches!(
            container.justify_content,
            JustifyContent::Start | JustifyContent::Default
        ) && matches!(
            container.align_items,
            AlignItems::Default | AlignItems::Start
        ) {
            return inner(ui, relative_container);
        }

        let contained_calculated_width = container.contained_calculated_width();
        let contained_calculated_height = container.contained_calculated_height();
        if container.justify_content == JustifyContent::End {
            ui.add_space(container.calculated_width.unwrap() - contained_calculated_width);
        }
        ui.allocate_new_ui(
            egui::UiBuilder::new().layout(match container.align_items {
                AlignItems::Center => {
                    egui::Layout::centered_and_justified(egui::Direction::TopDown)
                }
                AlignItems::End | AlignItems::Default | AlignItems::Start => {
                    match container.justify_content {
                        JustifyContent::Center => {
                            egui::Layout::top_down_justified(egui::Align::Center)
                        }
                        JustifyContent::End => egui::Layout::top_down_justified(egui::Align::Max),
                        _ => egui::Layout::top_down_justified(egui::Align::Min),
                    }
                }
            }),
            |ui| {
                egui::Frame::none().show(ui, |ui| {
                    ui.set_width(contained_calculated_width);
                    ui.set_height(contained_calculated_height);
                    if container.align_items == AlignItems::End {
                        let rect = egui::Rect::from_min_size(
                            ui.cursor().left_top(),
                            egui::vec2(
                                0.0,
                                container.calculated_height.unwrap() - contained_calculated_height,
                            ),
                        );
                        ui.advance_cursor_after_rect(rect);
                    }

                    inner(ui, relative_container)
                })
            },
        )
        .response
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn render_container_contents<'a>(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        container: &'a Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &'a Container)>,
        vscroll: bool,
    ) -> Response {
        Self::render_position(
            ui,
            container,
            relative_container,
            |ui, relative_container| {
                let mut frame = egui::Frame::none().inner_margin(egui::Margin {
                    left: container.calculated_padding_left.unwrap_or(0.0),
                    right: container.calculated_padding_right.unwrap_or(0.0),
                    top: container.calculated_padding_top.unwrap_or(0.0),
                    bottom: container.calculated_padding_bottom.unwrap_or(0.0),
                });

                if let Some(overrides) = render_context.backgrounds.get(&container.id) {
                    if let Some(StyleOverride {
                        value: Some(background),
                        ..
                    }) = overrides.first()
                    {
                        frame = frame.fill(background.into());
                    }
                } else if let Some(background) = container.background {
                    frame = frame.fill(background.into());
                }
                if container.calculated_border_top_left_radius.is_some()
                    || container.calculated_border_top_right_radius.is_some()
                    || container.calculated_border_bottom_left_radius.is_some()
                    || container.calculated_border_bottom_right_radius.is_some()
                {
                    frame = frame.rounding(egui::Rounding {
                        nw: container.calculated_border_top_left_radius.unwrap_or(0.0),
                        ne: container.calculated_border_top_right_radius.unwrap_or(0.0),
                        sw: container
                            .calculated_border_bottom_left_radius
                            .unwrap_or(0.0),
                        se: container
                            .calculated_border_bottom_right_radius
                            .unwrap_or(0.0),
                    });
                }

                frame
                    .show(ui, {
                        |ui| {
                            let width = container.calculated_width.unwrap();
                            let height = container.calculated_height.unwrap();

                            ui.set_width(width);
                            ui.set_height(height);

                            #[cfg(feature = "debug")]
                            let debug = *DEBUG.read().unwrap();

                            #[cfg(feature = "debug")]
                            let pos = if debug {
                                Some(ui.cursor().left_top())
                            } else {
                                None
                            };

                            if vscroll {
                                if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
                                    let rect = egui::Rect::from_pos(egui::emath::pos2(0.0, height));
                                    ui.scroll_to_rect(rect, Some(egui::Align::TOP));
                                }
                                if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
                                    let rect =
                                        egui::Rect::from_pos(egui::emath::pos2(0.0, -height));
                                    ui.scroll_to_rect(rect, Some(egui::Align::TOP));
                                }
                            }

                            let response = Self::render_layout(
                                ui,
                                container,
                                relative_container,
                                move |ui, relative_container| {
                                    self.render_direction(
                                        render_context,
                                        ctx,
                                        ui,
                                        container,
                                        viewport,
                                        rect,
                                        relative_container,
                                        vscroll,
                                    )
                                },
                            );

                            #[cfg(feature = "debug")]
                            if let Some(mut pos) = pos {
                                let painter = ui.painter();
                                let text = format!("({}, {}, {width}, {height})", pos.x, pos.y);
                                let galley = painter.layout_no_wrap(
                                    text.clone(),
                                    egui::FontId::default(),
                                    Color32::WHITE,
                                );
                                let rect = egui::Align2::LEFT_TOP.anchor_size(pos, galley.size());
                                painter.add(egui::Shape::rect_filled(rect, 0.0, Color32::WHITE));
                                ui.painter().text(
                                    pos,
                                    egui::Align2::LEFT_TOP,
                                    text,
                                    egui::FontId::default(),
                                    Color32::RED,
                                );

                                pos.y += rect.height();

                                if container.calculated_padding_left.is_some()
                                    || container.calculated_padding_right.is_some()
                                    || container.calculated_padding_top.is_some()
                                    || container.calculated_padding_bottom.is_some()
                                {
                                    let text = format!(
                                        "p({}, {}, {}, {})",
                                        container.calculated_padding_left.unwrap_or(0.0),
                                        container.calculated_padding_right.unwrap_or(0.0),
                                        container.calculated_padding_top.unwrap_or(0.0),
                                        container.calculated_padding_bottom.unwrap_or(0.0),
                                    );
                                    let galley = painter.layout_no_wrap(
                                        text.clone(),
                                        egui::FontId::default(),
                                        Color32::WHITE,
                                    );
                                    let rect =
                                        egui::Align2::LEFT_TOP.anchor_size(pos, galley.size());
                                    painter.add(egui::Shape::rect_filled(
                                        rect,
                                        0.0,
                                        Color32::WHITE,
                                    ));
                                    ui.painter().text(
                                        pos,
                                        egui::Align2::LEFT_TOP,
                                        text,
                                        egui::FontId::default(),
                                        Color32::RED,
                                    );

                                    pos.y += rect.height();
                                }

                                if container.calculated_margin_left.is_some()
                                    || container.calculated_margin_right.is_some()
                                    || container.calculated_margin_top.is_some()
                                    || container.calculated_margin_bottom.is_some()
                                {
                                    let text = format!(
                                        "m({}, {}, {}, {})",
                                        container.calculated_margin_left.unwrap_or(0.0),
                                        container.calculated_margin_right.unwrap_or(0.0),
                                        container.calculated_margin_top.unwrap_or(0.0),
                                        container.calculated_margin_bottom.unwrap_or(0.0),
                                    );
                                    let galley = painter.layout_no_wrap(
                                        text.clone(),
                                        egui::FontId::default(),
                                        Color32::WHITE,
                                    );
                                    let rect =
                                        egui::Align2::LEFT_TOP.anchor_size(pos, galley.size());
                                    painter.add(egui::Shape::rect_filled(
                                        rect,
                                        0.0,
                                        Color32::WHITE,
                                    ));
                                    ui.painter().text(
                                        pos,
                                        egui::Align2::LEFT_TOP,
                                        text,
                                        egui::FontId::default(),
                                        Color32::RED,
                                    );

                                    pos.y += rect.height();
                                }

                                if container.internal_padding_top.is_some()
                                    || container.internal_padding_right.is_some()
                                    || container.internal_padding_top.is_some()
                                    || container.internal_padding_bottom.is_some()
                                {
                                    let text = format!(
                                        "ip({}, {}, {}, {})",
                                        container.internal_padding_left.unwrap_or(0.0),
                                        container.internal_padding_right.unwrap_or(0.0),
                                        container.internal_padding_top.unwrap_or(0.0),
                                        container.internal_padding_bottom.unwrap_or(0.0),
                                    );
                                    let galley = painter.layout_no_wrap(
                                        text.clone(),
                                        egui::FontId::default(),
                                        Color32::WHITE,
                                    );
                                    let rect =
                                        egui::Align2::LEFT_TOP.anchor_size(pos, galley.size());
                                    painter.add(egui::Shape::rect_filled(
                                        rect,
                                        0.0,
                                        Color32::WHITE,
                                    ));
                                    ui.painter().text(
                                        pos,
                                        egui::Align2::LEFT_TOP,
                                        text,
                                        egui::FontId::default(),
                                        Color32::RED,
                                    );

                                    pos.y += rect.height();
                                }

                                if container.internal_margin_top.is_some()
                                    || container.internal_margin_right.is_some()
                                    || container.internal_margin_top.is_some()
                                    || container.internal_margin_bottom.is_some()
                                {
                                    let text = format!(
                                        "im({}, {}, {}, {})",
                                        container.internal_margin_left.unwrap_or(0.0),
                                        container.internal_margin_right.unwrap_or(0.0),
                                        container.internal_margin_top.unwrap_or(0.0),
                                        container.internal_margin_bottom.unwrap_or(0.0),
                                    );
                                    let galley = painter.layout_no_wrap(
                                        text.clone(),
                                        egui::FontId::default(),
                                        Color32::WHITE,
                                    );
                                    let rect =
                                        egui::Align2::LEFT_TOP.anchor_size(pos, galley.size());
                                    painter.add(egui::Shape::rect_filled(
                                        rect,
                                        0.0,
                                        Color32::WHITE,
                                    ));
                                    ui.painter().text(
                                        pos,
                                        egui::Align2::LEFT_TOP,
                                        text,
                                        egui::FontId::default(),
                                        Color32::RED,
                                    );

                                    pos.y += rect.height();
                                }
                            }

                            response
                        }
                    })
                    .response
            },
        )
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    fn render_elements(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        elements: &[Container],
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
        scroll_child: bool,
    ) {
        log::trace!("render_elements: {} elements", elements.len());
        for element in elements {
            self.render_element(
                render_context,
                ctx,
                ui,
                element,
                viewport,
                rect,
                relative_container,
                scroll_child,
            );
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    fn render_elements_ref(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        elements: &[&Container],
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
        scroll_child: bool,
    ) {
        log::trace!("render_elements_ref: {} elements", elements.len());
        for element in elements {
            self.render_element(
                render_context,
                ctx,
                ui,
                element,
                viewport,
                rect,
                relative_container,
                scroll_child,
            );
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn rect_contains_mouse(
        pointer: &egui::PointerState,
        rect: egui::Rect,
        viewport: Option<egui::Rect>,
    ) -> bool {
        pointer.latest_pos().is_some_and(|pos| {
            if viewport.is_some_and(|vp| !vp.contains(pos)) {
                return false;
            }
            rect.contains(pos)
        })
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::cognitive_complexity
    )]
    fn handle_container_side_effects(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: Option<&Ui>,
        container: &Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        response: Option<&Response>,
        recurse: bool,
    ) {
        self.handle_element_side_effects(ctx, &container.element, viewport, rect, response);

        if let Some(route) = &container.route {
            #[cfg(feature = "profiling")]
            profiling::scope!("route side effects");
            let processed_route = {
                render_context
                    .route_requests
                    .iter()
                    .any(|x| *x == container.id)
            };
            if !processed_route {
                log::debug!(
                    "processing route route={route:?} container_id={}",
                    container.id
                );
                render_context.route_requests.push(container.id);
                if let Err(e) = self.event.send(AppEvent::ProcessRoute {
                    route: route.to_owned(),
                    container_id: container.id,
                }) {
                    log::error!("Failed to send ProcessRoute event: {e:?}");
                }
            }
        }

        if let Some(response) = response {
            self.handle_ui_event_side_effects(container, ctx, viewport, rect, response);
        }

        self.handle_custom_event_side_effects(container);

        if let Some(ui) = ui {
            if let Element::Image {
                source: Some(source),
            } = &container.element
            {
                #[cfg(feature = "profiling")]
                profiling::scope!("image side effects");
                let pos = ui.cursor().left_top();
                let listener = render_context
                    .viewport_listeners
                    .entry(container.id)
                    .or_insert_with(|| {
                        ViewportListener::new(
                            viewport.cloned(),
                            0.0,
                            0.0,
                            container.calculated_width.unwrap(),
                            container.calculated_height.unwrap(),
                        )
                    });
                listener.viewport = viewport.cloned();
                listener.pos.x = pos.x + viewport.map_or(0.0, |x| x.viewport.x);
                listener.pos.y = pos.y + viewport.map_or(0.0, |x| x.viewport.y);

                let (_, (dist, prev_dist)) = listener.check();

                if prev_dist.is_none_or(|x| x >= 2000.0) && dist < 2000.0 {
                    let contains_image =
                        { matches!(render_context.images.get(source), Some(AppImage::Bytes(_))) };
                    if !contains_image {
                        let loading_image = {
                            matches!(render_context.images.get(source), Some(AppImage::Loading))
                        };

                        if !loading_image {
                            log::debug!(
                                "render_element: triggering LoadImage for source={source} ({}, {})",
                                listener.pos.x,
                                listener.pos.y
                            );
                            render_context
                                .images
                                .insert(source.clone(), AppImage::Loading);

                            if let Err(e) = self.event.send(AppEvent::LoadImage {
                                source: source.to_string(),
                            }) {
                                log::error!("Failed to send LoadImage event: {e:?}");
                            }
                        }
                    }
                }
            }
        }

        if recurse {
            for container in &container.children {
                self.handle_container_side_effects(
                    render_context,
                    ctx,
                    ui,
                    container,
                    viewport,
                    rect,
                    response,
                    recurse,
                );
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_ui_event_side_effects(
        &self,
        container: &Container,
        ctx: &egui::Context,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        response: &Response,
    ) {
        let viewport_rect = rect.map(|rect| {
            let (offset_x, offset_y) =
                viewport.map_or((0.0, 0.0), |viewport| (viewport.pos.x, viewport.pos.y));
            egui::Rect::from_min_size(egui::pos2(offset_x, offset_y), rect.size())
        });

        if let Some(cursor) = container.cursor {
            #[cfg(feature = "profiling")]
            profiling::scope!("cursor side effects");
            let ctx = ctx.clone();
            let pointer = ctx.input(|x| x.pointer.clone());
            let response = response.clone();
            self.trigger_side_effect(move |_render_context| {
                if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect) {
                    ctx.output_mut(|x| {
                        x.cursor_icon = cursor_to_cursor_icon(cursor);
                    });
                }

                true
            });
        }

        if container.is_visible() {
            for fx_action in &container.actions {
                let request_action = self.request_action.clone();

                match fx_action.trigger {
                    ActionTrigger::Click | ActionTrigger::ClickOutside => {
                        #[cfg(feature = "profiling")]
                        profiling::scope!("click/clickOutside side effects");
                        let inside = matches!(fx_action.trigger, ActionTrigger::Click);
                        let action = fx_action.action.clone();
                        let id = container.id;
                        let pointer = ctx.input(|x| x.pointer.clone());
                        let response = response.clone();
                        let sender = self.sender.clone();
                        self.trigger_side_effect(move |render_context| {
                            if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect)
                                == inside
                                && pointer.primary_released()
                            {
                                log::trace!("click action: {action}");
                                Self::handle_action(
                                    &action,
                                    StyleTrigger::UiEvent,
                                    render_context,
                                    id,
                                    &sender,
                                    &request_action,
                                    None,
                                );
                                return !inside;
                            }

                            Self::unhandle_action(
                                &action,
                                StyleTrigger::UiEvent,
                                render_context,
                                id,
                            );

                            true
                        });
                    }
                    ActionTrigger::Hover => {
                        #[cfg(feature = "profiling")]
                        profiling::scope!("hover side effects");
                        let action = fx_action.action.clone();
                        let id = container.id;
                        let response = response.clone();
                        let pointer = ctx.input(|x| x.pointer.clone());
                        let sender = self.sender.clone();
                        self.trigger_side_effect(move |render_context| {
                            if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect) {
                                log::trace!("hover action: {action}");
                                return Self::handle_action(
                                    &action,
                                    StyleTrigger::UiEvent,
                                    render_context,
                                    id,
                                    &sender,
                                    &request_action,
                                    None,
                                );
                            }

                            Self::unhandle_action(
                                &action,
                                StyleTrigger::UiEvent,
                                render_context,
                                id,
                            );

                            true
                        });
                    }
                    ActionTrigger::Change => {
                        #[cfg(feature = "profiling")]
                        profiling::scope!("change side effects");
                        let action = fx_action.action.clone();
                        let id = container.id;
                        let response = response.clone();
                        let sender = self.sender.clone();
                        self.trigger_side_effect(move |render_context| {
                            if response.changed() {
                                log::trace!("change action: {action}");
                                return Self::handle_action(
                                    &action,
                                    StyleTrigger::UiEvent,
                                    render_context,
                                    id,
                                    &sender,
                                    &request_action,
                                    None,
                                );
                            }

                            Self::unhandle_action(
                                &action,
                                StyleTrigger::UiEvent,
                                render_context,
                                id,
                            );

                            true
                        });
                    }
                    ActionTrigger::Immediate | ActionTrigger::Event(..) => {}
                }
            }
        }
    }

    fn handle_custom_event_side_effects(&self, container: &Container) {
        for fx_action in &container.actions {
            if let ActionTrigger::Event(event_name) = &fx_action.trigger {
                let request_action = self.request_action.clone();
                let action = fx_action.action.clone();
                let id = container.id;
                let sender = self.sender.clone();
                self.add_event_handler(event_name.to_string(), move |render_context, value| {
                    Self::handle_action(
                        &action,
                        StyleTrigger::CustomEvent,
                        render_context,
                        id,
                        &sender,
                        &request_action,
                        value,
                    );
                });
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    fn handle_element_side_effects(
        &self,
        ctx: &egui::Context,
        element: &Element,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        response: Option<&Response>,
    ) {
        log::trace!("handle_element_side_effects");
        if let Some(response) = response {
            #[cfg(feature = "profiling")]
            profiling::scope!("button and anchor side effects");
            let viewport_rect = rect.map(|rect| {
                let (offset_x, offset_y) =
                    viewport.map_or((0.0, 0.0), |viewport| (viewport.pos.x, viewport.pos.y));
                egui::Rect::from_min_size(egui::pos2(offset_x, offset_y), rect.size())
            });

            match element {
                Element::Button { .. } => {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("button side effects");
                    let response = response.clone();
                    let pointer = ctx.input(|x| x.pointer.clone());
                    let ctx = ctx.clone();
                    self.trigger_side_effect(move |_render_context| {
                        if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect) {
                            ctx.output_mut(|x| x.cursor_icon = CursorIcon::PointingHand);
                        }

                        true
                    });
                }
                Element::Anchor { href, .. } => {
                    #[cfg(feature = "profiling")]
                    profiling::scope!("anchor side effects");
                    let href = href.to_owned();
                    let sender = self.sender.clone();
                    let response = response.clone();
                    let pointer = ctx.input(|x| x.pointer.clone());
                    let ctx = ctx.clone();
                    self.trigger_side_effect(move |_render_context| {
                        if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect)
                            && pointer.primary_released()
                        {
                            if let Some(href) = href.clone() {
                                if let Err(e) = sender.send(href) {
                                    log::error!("Failed to send href event: {e:?}");
                                }
                            }
                        }

                        if Self::rect_contains_mouse(&pointer, response.rect, viewport_rect) {
                            ctx.output_mut(|x| x.cursor_icon = CursorIcon::PointingHand);
                        }

                        true
                    });
                }
                _ => {}
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn handle_action(
        action: &ActionType,
        trigger: StyleTrigger,
        render_context: &mut RenderContext,
        id: usize,
        sender: &Sender<String>,
        request_action: &Sender<String>,
        event_value: Option<&str>,
    ) -> bool {
        log::trace!("handle_action: action={action}");
        match &action {
            ActionType::Style { target, action } => Self::handle_style_action(
                action,
                target,
                trigger,
                id,
                render_context.container,
                render_context.visibilities,
                render_context.displays,
                render_context.backgrounds,
            ),
            ActionType::Navigate { url } => {
                if let Err(e) = sender.send(url.to_owned()) {
                    log::error!("Failed to navigate via action: {e:?}");
                }
                true
            }
            ActionType::Log { message, level } => {
                match level {
                    gigachad_actions::LogLevel::Error => {
                        log::error!("{message}");
                    }
                    gigachad_actions::LogLevel::Warn => {
                        log::warn!("{message}");
                    }
                    gigachad_actions::LogLevel::Info => {
                        log::info!("{message}");
                    }
                    gigachad_actions::LogLevel::Debug => {
                        log::debug!("{message}");
                    }
                    gigachad_actions::LogLevel::Trace => {
                        log::trace!("{message}");
                    }
                }

                true
            }
            ActionType::Custom { action } => {
                if let Err(e) = request_action.send(action.clone()) {
                    moosicbox_assert::die_or_error!("Failed to request action: {action} ({e:?})");
                }
                true
            }
            ActionType::Logic(eval) => {
                use gigachad_actions::logic::{CalcValue, Condition, Value};

                let calc_func = |calc_value: &CalcValue| match calc_value {
                    CalcValue::GetVisibility { target } => Some(Value::Visibility(
                        Self::map_element_target(target, id, render_context.container, |element| {
                            render_context
                                .visibilities
                                .get(&element.id)
                                .copied()
                                .or(element.visibility)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default(),
                    )),
                    CalcValue::GetId { target } => {
                        Self::map_element_target(target, id, render_context.container, |element| {
                            element.str_id.clone()
                        })
                        .flatten()
                        .map(Value::String)
                    }
                    CalcValue::GetDataAttrValue { attr, target } => {
                        Self::map_element_target(target, id, render_context.container, |element| {
                            element.data.get(attr).cloned()
                        })
                        .flatten()
                        .map(Value::String)
                    }
                    CalcValue::GetEventValue => {
                        event_value.map(ToString::to_string).map(Value::String)
                    }
                };

                let success = match &eval.condition {
                    Condition::Eq(a, b) => {
                        log::debug!("handle_action: checking eq a={a:?} b={b:?}");

                        if let Value::Calc(calc_value) = a {
                            if let Value::Calc(b_calc_value) = b {
                                let a = calc_func(calc_value);
                                let b = calc_func(b_calc_value);
                                log::debug!("handle_action: inner checking eq a={a:?} b={b:?}");
                                a == b
                            } else {
                                calc_func(calc_value).is_some_and(|a| {
                                    log::debug!("handle_action: inner checking eq a={a:?} b={b:?}");
                                    &a == b
                                })
                            }
                        } else if let Value::Calc(calc_value) = b {
                            calc_func(calc_value).is_some_and(|b| {
                                log::debug!("handle_action: inner checking eq a={a:?} b={b:?}");
                                a == &b
                            })
                        } else {
                            log::debug!("handle_action: inner checking eq a={a:?} b={b:?}");
                            a == b
                        }
                    }
                };

                log::debug!("handle_action: success={success}");

                if success {
                    for action in &eval.actions {
                        if matches!(
                            action.trigger,
                            ActionTrigger::Immediate | ActionTrigger::Event(..)
                        ) && !Self::handle_action(
                            &action.action,
                            trigger,
                            render_context,
                            id,
                            sender,
                            request_action,
                            event_value,
                        ) {
                            return false;
                        }
                    }
                } else {
                    for action in &eval.else_actions {
                        if matches!(
                            action.trigger,
                            ActionTrigger::Immediate | ActionTrigger::Event(..)
                        ) && !Self::handle_action(
                            &action.action,
                            trigger,
                            render_context,
                            id,
                            sender,
                            request_action,
                            event_value,
                        ) {
                            return false;
                        }
                    }
                }

                true
            }
            ActionType::Event { name, .. } => {
                log::trace!("handle_action: event '{name}' will be handled elsewhere");
                true
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn unhandle_action(
        action: &ActionType,
        trigger: StyleTrigger,
        render_context: &mut RenderContext,
        id: usize,
    ) {
        if let ActionType::Style { action, target } = &action {
            match action {
                StyleAction::SetVisibility { .. } => {
                    if let Some(id) =
                        Self::get_element_target_id(target, id, render_context.container)
                    {
                        if render_context.visibilities.contains_key(&id) {
                            render_context.visibilities.remove(&id);
                        }
                    }
                }
                StyleAction::SetDisplay { .. } => {
                    if let Some(id) =
                        Self::get_element_target_id(target, id, render_context.container)
                    {
                        if render_context.displays.contains_key(&id) {
                            render_context.displays.remove(&id);
                        }
                    }
                }
                StyleAction::SetBackground(..) => {
                    if let Some(id) =
                        Self::get_element_target_id(target, id, render_context.container)
                    {
                        if let Some(overrides) = render_context.backgrounds.get_mut(&id) {
                            overrides.retain(|x| x.trigger != trigger);

                            // TODO: don't delete the corresponding entry. just check for if it
                            // is empty in places this is read from.
                            if overrides.is_empty() {
                                render_context.backgrounds.remove(&id);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn map_element_target<R>(
        target: &ElementTarget,
        self_id: usize,
        container: &Container,
        func: impl Fn(&Container) -> R,
    ) -> Option<R> {
        match target {
            ElementTarget::StrId(str_id) => {
                if let Some(element) = container.find_element_by_str_id(str_id) {
                    return Some(func(element));
                }

                log::warn!("Could not find element with str id '{str_id}'");
            }
            ElementTarget::Id(id) => {
                if let Some(element) = container.find_element_by_id(self_id) {
                    return Some(func(element));
                }

                log::warn!("Could not find element with id '{id}'");
            }
            ElementTarget::SelfTarget => {
                if let Some(element) = container.find_element_by_id(self_id) {
                    return Some(func(element));
                }

                log::warn!("Could not find element with id '{self_id}'");
            }
            ElementTarget::LastChild => {
                if let Some(element) = container
                    .find_element_by_id(self_id)
                    .and_then(|x| x.children.iter().last())
                {
                    return Some(func(element));
                }

                log::warn!("Could not find element last child for id '{self_id}'");
            }
        }

        None
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_element_target_id(
        target: &ElementTarget,
        self_id: usize,
        container: &Container,
    ) -> Option<usize> {
        match target {
            ElementTarget::StrId(str_id) => {
                if let Some(element) = container.find_element_by_str_id(str_id) {
                    return Some(element.id);
                }

                log::warn!("Could not find element with str id '{str_id}'");
            }
            ElementTarget::Id(id) => {
                return Some(*id);
            }
            ElementTarget::SelfTarget => {
                return Some(self_id);
            }
            ElementTarget::LastChild => {
                if let Some(element) = container
                    .find_element_by_id(self_id)
                    .and_then(|x| x.children.iter().last())
                {
                    return Some(element.id);
                }

                log::warn!("Could not find element last child for id '{self_id}'");
            }
        }

        None
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments)]
    fn handle_style_action(
        action: &StyleAction,
        target: &ElementTarget,
        trigger: StyleTrigger,
        id: usize,
        container: &Container,
        visibilities: &mut HashMap<usize, Visibility>,
        displays: &mut HashMap<usize, bool>,
        backgrounds: &mut HashMap<usize, Vec<StyleOverride<Option<Color>>>>,
    ) -> bool {
        match action {
            StyleAction::SetVisibility(visibility) => {
                if let Some(id) = Self::get_element_target_id(target, id, container) {
                    visibilities.insert(id, *visibility);
                }

                true
            }
            StyleAction::SetDisplay(display) => {
                if let Some(id) = Self::get_element_target_id(target, id, container) {
                    displays.insert(id, *display);
                }

                true
            }
            StyleAction::SetBackground(background) => {
                if let Some(id) = Self::get_element_target_id(target, id, container) {
                    if let Some(background) = background {
                        match Color::try_from_hex(background) {
                            Ok(color) => {
                                log::debug!("handle_style_action: set background color id={id} color={color} trigger={trigger:?}");
                                let style_override = StyleOverride {
                                    trigger,
                                    value: Some(color),
                                };
                                match backgrounds.entry(id) {
                                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                                        entry.get_mut().push(style_override);
                                    }
                                    std::collections::hash_map::Entry::Vacant(entry) => {
                                        entry.insert(vec![style_override]);
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("handle_style_action: invalid background color: {e:?}");
                            }
                        }
                    } else {
                        log::debug!("handle_style_action: remove background color id={id} trigger={trigger:?}");
                        let style_override = StyleOverride {
                            trigger,
                            value: None,
                        };
                        match backgrounds.entry(id) {
                            std::collections::hash_map::Entry::Occupied(mut entry) => {
                                entry.get_mut().push(style_override);
                            }
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                entry.insert(vec![style_override]);
                            }
                        }
                    }
                }

                true
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn render_element(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        element: &Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
        scroll_child: bool,
    ) {
        log::trace!("render_element: rect={rect:?}");

        if element.element == Element::Table {
            self.render_table(
                render_context,
                ctx,
                ui,
                element,
                viewport,
                rect,
                relative_container,
            );
            return;
        }

        if scroll_child
            && self.handle_scroll_child_out_of_view(
                render_context,
                ctx,
                ui,
                element,
                viewport,
                rect,
                relative_container,
            )
        {
            return;
        }

        let response = match &element.element {
            Element::Input { input } => {
                Some(Self::render_input(ui, input, render_context.checkboxes))
            }
            Element::Raw { value } => Some(ui.label(value)),
            Element::Image { source } => source
                .as_ref()
                .map(|source| Self::render_image(render_context, ui, source, element)),
            Element::Canvas => element.str_id.as_ref().map_or_else(
                || None,
                |str_id| Self::render_canvas(render_context, ui, str_id, element),
            ),
            _ => None,
        };

        if let Some(response) = response {
            self.handle_container_side_effects(
                render_context,
                ctx,
                Some(ui),
                element,
                viewport,
                rect,
                Some(&response),
                false,
            );
            return;
        }

        self.render_container(
            render_context,
            ctx,
            ui,
            element,
            viewport,
            rect,
            relative_container,
        );
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_input(
        ui: &mut Ui,
        input: &Input,
        checkboxes: &mut HashMap<egui::Id, bool>,
    ) -> Response {
        match input {
            Input::Text { .. } | Input::Password { .. } => Self::render_text_input(ui, input),
            Input::Checkbox { .. } => Self::render_checkbox_input(ui, input, checkboxes),
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_text_input(ui: &mut Ui, input: &Input) -> Response {
        let (Input::Text { value, .. } | Input::Password { value, .. }) = input else {
            unreachable!()
        };

        let id = ui.next_auto_id();
        let mut value_text = ui
            .data_mut(|data| data.remove_temp::<String>(id))
            .unwrap_or_else(|| value.clone().unwrap_or_default());
        let mut text_edit = egui::TextEdit::singleline(&mut value_text).id(id);

        if let Input::Password { .. } = input {
            text_edit = text_edit.password(true);
        }

        let response = text_edit.ui(ui);
        ui.data_mut(|data| data.insert_temp(id, value_text));
        response
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_checkbox_input(
        ui: &mut Ui,
        input: &Input,
        checkboxes: &mut HashMap<egui::Id, bool>,
    ) -> Response {
        let Input::Checkbox { checked } = input else {
            unreachable!();
        };
        let checked = *checked;

        let id = ui.next_auto_id();

        let contains = { checkboxes.contains_key(&id) };

        let mut checked_value = ui
            .data_mut(|data| {
                let value = data.remove_temp::<bool>(id);

                if !contains {
                    return None;
                }

                value
            })
            .unwrap_or_else(|| checked.unwrap_or_default());

        let checkbox = egui::Checkbox::without_text(&mut checked_value);
        let response = checkbox.ui(ui);

        ui.data_mut(|data| data.insert_temp(id, checked_value));

        if response.changed() {
            checkboxes.insert(id, checked_value);
        }

        response
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_image(
        render_context: &mut RenderContext,
        ui: &mut Ui,
        source: &str,
        container: &Container,
    ) -> Response {
        egui::Frame::none()
            .show(ui, |ui| {
                ui.set_width(container.calculated_width.unwrap());
                ui.set_height(container.calculated_height.unwrap());

                let Some(AppImage::Bytes(bytes)) = render_context.images.get(source).cloned()
                else {
                    return;
                };

                log::trace!(
                    "render_image: showing image for source={source} ({}, {})",
                    container.calculated_width.unwrap(),
                    container.calculated_height.unwrap(),
                );

                egui::Image::from_bytes(source.to_string(), egui::load::Bytes::Shared(bytes))
                    .max_width(container.calculated_width.unwrap())
                    .max_height(container.calculated_height.unwrap())
                    .ui(ui);
            })
            .response
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_canvas(
        render_context: &mut RenderContext,
        ui: &mut Ui,
        str_id: &str,
        container: &Container,
    ) -> Option<Response> {
        render_context.canvas_actions.get(str_id).map_or_else(
            || None,
            |actions| {
                let (response, painter) = ui.allocate_painter(
                    egui::Vec2::new(
                        container.calculated_width.unwrap(),
                        container.calculated_height.unwrap(),
                    ),
                    egui::Sense::hover(),
                );

                let pixels_per_point = 1.0; // ctx.pixels_per_point();
                let cursor_px = egui::Pos2::new(
                    response.rect.min.x * pixels_per_point,
                    response.rect.min.y * pixels_per_point,
                )
                .ceil();

                let default_color = Color32::BLACK;
                let stroke = &mut egui::epaint::PathStroke::new(1.0, default_color).inside();
                stroke.color = egui::epaint::ColorMode::Solid(default_color);

                for action in actions {
                    match action {
                        CanvasAction::Clear => {}
                        CanvasAction::StrokeSize(size) => {
                            stroke.width = *size;
                        }
                        CanvasAction::StrokeColor(color) => {
                            stroke.color = egui::epaint::ColorMode::Solid((*color).into());
                        }
                        CanvasAction::Line(start, end) => {
                            painter.line_segment(
                                [
                                    egui::Pos2::new(start.0 + cursor_px.x, start.1 + cursor_px.y),
                                    egui::Pos2::new(end.0 + cursor_px.x, end.1 + cursor_px.y),
                                ],
                                stroke.clone(),
                            );
                        }
                        CanvasAction::FillRect(start, end) => {
                            let egui::epaint::ColorMode::Solid(color) = stroke.color else {
                                continue;
                            };
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::Pos2::new(start.0 + cursor_px.x, start.1 + cursor_px.y),
                                    egui::Pos2::new(end.0 + cursor_px.x, end.1 + cursor_px.y),
                                ),
                                0.0,
                                color,
                            );
                        }
                    }
                }

                Some(response)
            },
        )
    }

    fn apply_container_styles(
        container: &Container,
        mut frame: egui::Frame,
        start: bool,
        end: bool,
        backgrounds: &HashMap<usize, Vec<StyleOverride<Option<Color>>>>,
    ) -> egui::Frame {
        if let Some(overrides) = backgrounds.get(&container.id) {
            if let Some(StyleOverride {
                value: Some(background),
                ..
            }) = overrides.first()
            {
                frame = frame.fill(background.into());
            }
        } else if let Some(background) = container.background {
            frame = frame.fill(background.into());
        }

        if start && end {
            if container.calculated_border_top_left_radius.is_some()
                || container.calculated_border_top_right_radius.is_some()
                || container.calculated_border_bottom_left_radius.is_some()
                || container.calculated_border_bottom_right_radius.is_some()
            {
                frame = frame.rounding(egui::Rounding {
                    nw: container.calculated_border_top_left_radius.unwrap_or(0.0),
                    ne: container.calculated_border_top_right_radius.unwrap_or(0.0),
                    sw: container
                        .calculated_border_bottom_left_radius
                        .unwrap_or(0.0),
                    se: container
                        .calculated_border_bottom_right_radius
                        .unwrap_or(0.0),
                });
            }
        } else if start {
            if container.calculated_border_top_left_radius.is_some()
                || container.calculated_border_bottom_left_radius.is_some()
            {
                frame = frame.rounding(egui::Rounding {
                    nw: container.calculated_border_top_left_radius.unwrap_or(0.0),
                    ne: 0.0,
                    sw: container
                        .calculated_border_bottom_left_radius
                        .unwrap_or(0.0),
                    se: 0.0,
                });
            }
        } else if end
            && (container.calculated_border_top_right_radius.is_some()
                || container.calculated_border_bottom_right_radius.is_some())
        {
            frame = frame.rounding(egui::Rounding {
                nw: 0.0,
                ne: container.calculated_border_top_right_radius.unwrap_or(0.0),
                sw: 0.0,
                se: container
                    .calculated_border_bottom_right_radius
                    .unwrap_or(0.0),
            });
        }

        frame
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    #[cfg_attr(feature = "profiling", profiling::function)]
    fn render_table(
        &self,
        render_context: &mut RenderContext,
        ctx: &egui::Context,
        ui: &mut Ui,
        element: &Container,
        viewport: Option<&Viewport>,
        rect: Option<egui::Rect>,
        relative_container: Option<(egui::Rect, &Container)>,
    ) {
        let TableIter { rows, headings } = element.table_iter();

        let mut head_trs = element
            .children
            .iter()
            .filter(|x| matches!(x.element, Element::THead))
            .flat_map(|x| x.children.iter());

        let mut body_trs = element
            .children
            .iter()
            .filter(|x| matches!(x.element, Element::TBody))
            .flat_map(|x| x.children.iter());

        let grid = egui::Grid::new(format!("grid-{}", element.id));

        grid.show(ui, |ui| {
            if let Some(headings) = headings {
                for heading in headings {
                    let tr = head_trs.next();
                    if let Some(tr) = tr {
                        self.handle_container_side_effects(
                            render_context,
                            ctx,
                            Some(ui),
                            tr,
                            viewport,
                            rect,
                            None,
                            false,
                        );
                    }
                    let mut heading = heading.peekable();
                    let mut first = true;
                    while let Some(th) = heading.next() {
                        let mut frame = egui::Frame::none();

                        if let Some(tr) = tr {
                            frame = Self::apply_container_styles(
                                tr,
                                frame,
                                first,
                                heading.peek().is_none(),
                                render_context.backgrounds,
                            );
                        }

                        let response = frame.show(ui, |ui| {
                            self.render_container(
                                render_context,
                                ctx,
                                ui,
                                th,
                                viewport,
                                rect,
                                relative_container,
                            );
                        });

                        if let Some(tr) = tr {
                            self.handle_ui_event_side_effects(
                                tr,
                                ctx,
                                viewport,
                                rect,
                                &response.response,
                            );
                        }

                        first = false;
                    }
                    ui.end_row();
                }
            }
            for row in rows {
                let tr = body_trs.next();
                if let Some(tr) = tr {
                    self.handle_container_side_effects(
                        render_context,
                        ctx,
                        Some(ui),
                        tr,
                        viewport,
                        rect,
                        None,
                        false,
                    );
                }
                {
                    let mut row = row.peekable();
                    let mut first = true;
                    while let Some(td) = row.next() {
                        let mut frame = egui::Frame::none();

                        if let Some(tr) = tr {
                            frame = Self::apply_container_styles(
                                tr,
                                frame,
                                first,
                                row.peek().is_none(),
                                render_context.backgrounds,
                            );
                        }

                        let response = frame.show(ui, |ui| {
                            self.render_container(
                                render_context,
                                ctx,
                                ui,
                                td,
                                viewport,
                                rect,
                                relative_container,
                            );
                        });

                        if let Some(tr) = tr {
                            self.handle_ui_event_side_effects(
                                tr,
                                ctx,
                                viewport,
                                rect,
                                &response.response,
                            );
                        }

                        first = false;
                    }
                    ui.end_row();
                }
            }
        });
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn add_event_handler(
        &self,
        event_name: String,
        handler: impl Fn(&mut RenderContext, Option<&str>) + Send + Sync + 'static,
    ) {
        self.event_handlers
            .write()
            .unwrap()
            .push((event_name, Box::new(handler)));
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn trigger_side_effect(
        &self,
        handler: impl Fn(&mut RenderContext) -> bool + Send + Sync + 'static,
    ) {
        self.side_effects
            .lock()
            .unwrap()
            .push_back(Box::new(handler));
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn paint(&self, ctx: &egui::Context) {
        self.check_frame_resize(ctx);

        self.event_handlers.write().unwrap().clear();

        let container = self.container.write().unwrap();
        let mut viewport_listeners = self.viewport_listeners.write().unwrap();
        let mut images = self.images.write().unwrap();
        let mut canvas_actions = self.canvas_actions.write().unwrap();
        let mut route_requests = self.route_requests.write().unwrap();
        let mut visibilities = self.visibilities.write().unwrap();
        let mut backgrounds = self.backgrounds.write().unwrap();
        let mut displays = self.displays.write().unwrap();
        let mut checkboxes = self.checkboxes.write().unwrap();

        #[cfg(feature = "debug")]
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            let value = {
                let mut handle = DEBUG.write().unwrap();
                let value = *handle;
                let value = !value;
                *handle = value;
                value
            };
            log::debug!("Set DEBUG to {value}");
        }

        {
            let mut render_context = RenderContext {
                container: &container,
                viewport_listeners: &mut viewport_listeners,
                images: &mut images,
                canvas_actions: &mut canvas_actions,
                route_requests: &mut route_requests,
                visibilities: &mut visibilities,
                backgrounds: &mut backgrounds,
                displays: &mut displays,
                checkboxes: &mut checkboxes,
            };

            ctx.memory_mut(|x| {
                x.options.line_scroll_speed = 100.0;
            });

            ctx.style_mut(|style| {
                style.spacing.window_margin.left = 0.0;
                style.spacing.window_margin.right = 0.0;
                style.spacing.window_margin.top = 0.0;
                style.spacing.window_margin.bottom = 0.0;
                style.spacing.item_spacing = egui::emath::Vec2::splat(0.0);
                #[cfg(all(debug_assertions, feature = "debug"))]
                {
                    style.debug.debug_on_hover = true;
                }
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(
                            self.background
                                .unwrap_or_else(|| Color32::from_hex("#181a1b").unwrap()),
                        )
                        .show(ui, |ui| {
                            self.render_container(
                                &mut render_context,
                                ctx,
                                ui,
                                &container,
                                None,
                                None,
                                None,
                            );
                        });
                });

            let mut handlers_count = 0;

            for handler in self.side_effects.lock().unwrap().drain(..) {
                handlers_count += 1;
                if !handler(&mut render_context) {
                    break;
                }
            }

            log::trace!("paint: {handlers_count} handler(s) on render");
        }

        drop(container);
        drop(viewport_listeners);
        drop(images);
        drop(canvas_actions);
        drop(route_requests);
        drop(visibilities);
        drop(displays);
        drop(checkboxes);

        #[cfg(feature = "profiling")]
        profiling::finish_frame!();
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.paint(ctx);
    }
}

const fn cursor_to_cursor_icon(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Auto => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::PointingHand,
        Cursor::Text => CursorIcon::Text,
        Cursor::Crosshair => CursorIcon::Crosshair,
        Cursor::Move => CursorIcon::Move,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::NoDrop => CursorIcon::NoDrop,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::AllScroll => CursorIcon::AllScroll,
        Cursor::ColResize => CursorIcon::ResizeColumn,
        Cursor::RowResize => CursorIcon::ResizeRow,
        Cursor::NResize => CursorIcon::ResizeNorth,
        Cursor::EResize => CursorIcon::ResizeEast,
        Cursor::SResize => CursorIcon::ResizeSouth,
        Cursor::WResize => CursorIcon::ResizeWest,
        Cursor::NeResize => CursorIcon::ResizeNorthEast,
        Cursor::NwResize => CursorIcon::ResizeNorthWest,
        Cursor::SeResize => CursorIcon::ResizeSouthEast,
        Cursor::SwResize => CursorIcon::ResizeSouthWest,
        Cursor::EwResize => CursorIcon::ResizeHorizontal,
        Cursor::NsResize => CursorIcon::ResizeVertical,
        Cursor::NeswResize => CursorIcon::ResizeNwSe,
        Cursor::ZoomIn => CursorIcon::ZoomIn,
        Cursor::ZoomOut => CursorIcon::ZoomOut,
    }
}

#[cfg(feature = "profiling-puffin")]
fn start_puffin_server() {
    puffin::set_scopes_on(true);

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    }
}
