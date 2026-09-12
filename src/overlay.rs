use std::sync::mpsc::channel;
use std::time::Instant;

use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::SlotPool,
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface::WlSurface},
    Connection, QueueHandle,
};

use crate::command::Command;
use crate::socket::TriggerSocket;
use crate::sprite::Sprite;
use crate::state_machine::{AnimationConfig, OverlayStateMachine};

pub struct OverlayConfig {
    pub surface_width: u32,
    pub surface_height: u32,
    pub margin_bottom: i32,
    pub margin_right: i32,
    pub animation: AnimationConfig,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            surface_width: 256,
            surface_height: 256,
            margin_bottom: 16,
            margin_right: 16,
            animation: AnimationConfig::default(),
        }
    }
}

pub struct AppState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,

    pub layer_surface: Option<LayerSurface>,
    pub pool: Option<SlotPool>,
    pub empty_region: Option<Region>,
    pub sprite: Sprite,
    pub state_machine: OverlayStateMachine,
    pub config: OverlayConfig,

    pub configured: bool,
    pub frame_pending: bool,
    pub exit: bool,
}

delegate_registry!(AppState);
delegate_output!(AppState);
delegate_compositor!(AppState);
delegate_shm!(AppState);
delegate_layer!(AppState);

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl CompositorHandler for AppState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: wayland_client::protocol::wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &WlSurface, _: u32) {
        self.frame_pending = false;
        let now = Instant::now();
        let _ = self.state_machine.update(now);
        self.render(qh);
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl LayerShellHandler for AppState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }
    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &LayerSurface,
        _: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let first_configure = !self.configured;
        self.configured = true;

        if first_configure {
            self.render(qh);
        }
    }
}

impl AppState {
    pub fn handle_command(&mut self, cmd: Command, qh: &QueueHandle<Self>) {
        let now = Instant::now();
        match cmd {
            Command::Show => {
                self.state_machine.show(now);
                self.request_render(qh);
            }
            Command::Hide => {
                self.state_machine.hide(now);
                self.request_render(qh);
            }
            Command::Toggle => {
                self.state_machine.toggle(now);
                self.request_render(qh);
            }
            Command::Status => {
                // Status is queried; if in a terminal or socket client, output is given
            }
            Command::Quit => {
                self.exit = true;
            }
        }
    }

    pub fn request_render(&mut self, qh: &QueueHandle<Self>) {
        if !self.configured {
            return;
        }
        if !self.frame_pending {
            self.render(qh);
        }
    }

    pub fn render(&mut self, qh: &QueueHandle<Self>) {
        if !self.configured {
            return;
        }

        let width = self.config.surface_width;
        let height = self.config.surface_height;
        let stride = (width * 4) as i32;

        let pool = match self.pool.as_mut() {
            Some(p) => p,
            None => return,
        };

        let (buffer, canvas) = match pool.create_buffer(
            width as i32,
            height as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Failed to create SHM buffer: {:?}", e);
                return;
            }
        };

        // Clear canvas to completely transparent
        canvas.fill(0);

        let now = Instant::now();
        if !self.state_machine.is_hidden() {
            let current_y = self.state_machine.current_y(now);
            let dest_x = ((width as i32 - self.sprite.width as i32) / 2).max(0);
            let dest_y = current_y as i32;

            self.sprite
                .blit_to_canvas(canvas, width, height, dest_x, dest_y);
        }

        let layer_surface = match self.layer_surface.as_ref() {
            Some(s) => s,
            None => return,
        };

        layer_surface
            .wl_surface()
            .attach(Some(buffer.wl_buffer()), 0, 0);
        layer_surface
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);

        // If animation in progress, request next frame callback
        if self.state_machine.is_animating() {
            layer_surface
                .wl_surface()
                .frame(qh, layer_surface.wl_surface().clone());
            self.frame_pending = true;
        }

        layer_surface.commit();
    }
}

pub fn run_overlay(
    sprite: Sprite,
    config: OverlayConfig,
    socket_path: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let shm_state = Shm::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);

    let surface = compositor_state.create_surface(&qh);
    let layer_surface = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Overlay,
        Some("felt-overlay"),
        None,
    );

    layer_surface.set_anchor(Anchor::BOTTOM | Anchor::RIGHT);
    layer_surface.set_size(config.surface_width, config.surface_height);
    layer_surface.set_margin(0, config.margin_right, config.margin_bottom, 0);
    layer_surface.set_exclusive_zone(0);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

    // Click-through: configure an empty input region so mouse events pass through completely
    let empty_region = Region::new(&compositor_state)?;
    layer_surface.set_input_region(Some(empty_region.wl_region()));
    layer_surface.commit();

    let pool_len = (config.surface_width * config.surface_height * 4 * 2) as usize;
    let pool = SlotPool::new(pool_len, &shm_state)?;

    let state_machine = OverlayStateMachine::new(config.animation);

    let mut state = AppState {
        registry_state,
        output_state,
        compositor_state,
        shm_state,
        layer_shell,
        layer_surface: Some(layer_surface),
        pool: Some(pool),
        empty_region: Some(empty_region),
        sprite,
        state_machine,
        config,
        configured: false,
        frame_pending: false,
        exit: false,
    };

    let target_sock = socket_path.unwrap_or_else(crate::socket::default_socket_path);
    let (cmd_tx, cmd_rx) = channel();
    let trigger_socket = TriggerSocket::bind(&target_sock, cmd_tx)?;
    println!(
        "felt-overlay listening on socket: {}",
        trigger_socket.path().display()
    );

    let mut event_loop: EventLoop<AppState> = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();

    WaylandSource::new(conn, event_queue).insert(loop_handle)?;

    while !state.exit {
        // Drain any socket commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            state.handle_command(cmd, &qh);
        }

        let now = Instant::now();
        if state.state_machine.update(now) {
            state.request_render(&qh);
        }

        let timeout = state
            .state_machine
            .next_deadline()
            .map(|d| d.saturating_duration_since(Instant::now()))
            .unwrap_or(std::time::Duration::from_millis(100)); // check socket queue periodically

        event_loop.dispatch(Some(timeout), &mut state)?;
    }

    println!("felt-overlay shutting down cleanly.");
    Ok(())
}
