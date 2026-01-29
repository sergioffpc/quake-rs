use clap::Parser;
use quake_input::Source;
use quake_world::world::{WorldId, WorldMode};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "config/")]
    config_path: PathBuf,
    #[arg(long, default_value = "res/")]
    resources_path: PathBuf,
    #[arg(long, default_value = "certs/")]
    certs_path: PathBuf,
    #[arg(long, default_value = "[::1]:30512")]
    connect_addr: SocketAddr,
}

pub async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::<ClientCommand>::with_user_event().build()?;

    // Applications which want to render at the display’s native refresh rate should use Poll and
    // the VSync functionality of a graphics API to reduce odds of missed frames.
    event_loop.set_control_flow(ControlFlow::Poll);

    let event_loop_proxy = event_loop.create_proxy();

    let args = Args::parse();
    let mut app = ClientApp::new(args, event_loop_proxy).await?;

    event_loop.run_app(&mut app).map_err(Into::into)
}

#[derive(Clone, Debug)]
enum ClientCommand {
    Spawn { world_mode: WorldMode },
    Despawn,
    Join { world_id: WorldId },
    Leave,
    Halt,
}

impl ClientCommand {
    fn from_str(s: &str) -> Option<Self> {
        let mut iter = s.split_whitespace();
        match iter.next() {
            Some("app.spawn") => match iter.next() {
                Some("demo") => Some(Self::Spawn {
                    world_mode: WorldMode::Demo(PathBuf::from(iter.next().unwrap())),
                }),
                Some("campaign") => Some(Self::Spawn {
                    world_mode: WorldMode::Campaign(PathBuf::from(iter.next().unwrap())),
                }),
                Some("deathmatch") => Some(Self::Spawn {
                    world_mode: WorldMode::Deathmatch(PathBuf::from(iter.next().unwrap())),
                }),
                _ => None,
            },
            Some("app.despawn") => Some(Self::Despawn),
            Some("app.join") => Some(Self::Join {
                world_id: WorldId::from(iter.next().unwrap().parse::<u64>().unwrap()),
            }),
            Some("app.leave") => Some(Self::Leave),
            Some("app.halt") => Some(Self::Halt),
            _ => None,
        }
    }
}

#[derive(Default)]
enum ClientPhase {
    Initialized {
        window: winit::window::Window,
        render_manager: quake_render::RenderManager,
    },
    #[default]
    Uninitialized,
}

struct ClientApp {
    event_loop_proxy: EventLoopProxy<ClientCommand>,
    phase: ClientPhase,

    audio_manager: quake_audio::AudioManager,
    input_manager: quake_input::InputManager,
    world_manager: quake_world::world::WorldClient,
}

impl ClientApp {
    async fn new(
        args: Args,
        event_loop_proxy: EventLoopProxy<ClientCommand>,
    ) -> anyhow::Result<Self> {
        let audio_manager = quake_audio::AudioManager::new()?;
        let bindings_path = args.config_path.to_path_buf().join("bindings.toml");
        let mappings_path = args.config_path.to_path_buf().join("mappings.toml");
        let input_manager = quake_input::InputManager::default()
            .with_bindings(bindings_path)
            .unwrap()
            .with_mappings(mappings_path)?;
        let network_manager =
            quake_network::NetworkClient::quic(args.connect_addr, args.certs_path).await?;
        let asset_manager = quake_asset::AssetManager::new(args.resources_path)?;
        let world_manager =
            quake_world::world::WorldClient::new(network_manager, asset_manager).await?;

        Ok(Self {
            event_loop_proxy,
            phase: ClientPhase::Uninitialized,
            audio_manager,
            input_manager,
            world_manager,
        })
    }
}

impl ApplicationHandler<ClientCommand> for ClientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let ClientPhase::Uninitialized = self.phase {
            let width = 1280;
            let height = 720;
            let window_attributes = winit::window::WindowAttributes::default()
                .with_decorations(false)
                .with_inner_size(LogicalSize::new(width, height));
            let window = event_loop.create_window(window_attributes).unwrap();

            use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
            let render_manager =
                quake_render::RenderManager::new(&quake_render::RenderManagerDescriptor {
                    display_handle: &window.display_handle().unwrap(),
                    window_handle: &window.window_handle().unwrap(),
                    width,
                    height,
                })
                .unwrap();
            self.phase = ClientPhase::Initialized {
                window,
                render_manager,
            };
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ClientCommand) {
        info!(?event, "user event");

        match event {
            ClientCommand::Spawn { world_mode } => self.world_manager.spawn(world_mode).unwrap(),
            ClientCommand::Despawn => self.world_manager.despawn().unwrap(),
            ClientCommand::Join { world_id } => self.world_manager.join(world_id).unwrap(),
            ClientCommand::Leave => self.world_manager.leave().unwrap(),
            ClientCommand::Halt => {
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let ClientPhase::Initialized {
            window,
            render_manager,
        } = &mut self.phase
        else {
            return;
        };

        match event {
            WindowEvent::Resized(_) => {
                render_manager.on_resize(window.inner_size().width, window.inner_size().height);
            }
            WindowEvent::Moved(_) => {}
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                let key = match key_code {
                    KeyCode::Backquote => quake_input::Key::Backquote,
                    KeyCode::Backslash => quake_input::Key::Backslash,
                    KeyCode::BracketLeft => quake_input::Key::BracketLeft,
                    KeyCode::BracketRight => quake_input::Key::BracketRight,
                    KeyCode::Comma => quake_input::Key::Comma,
                    KeyCode::Digit0 => quake_input::Key::Digit0,
                    KeyCode::Digit1 => quake_input::Key::Digit1,
                    KeyCode::Digit2 => quake_input::Key::Digit2,
                    KeyCode::Digit3 => quake_input::Key::Digit3,
                    KeyCode::Digit4 => quake_input::Key::Digit4,
                    KeyCode::Digit5 => quake_input::Key::Digit5,
                    KeyCode::Digit6 => quake_input::Key::Digit6,
                    KeyCode::Digit7 => quake_input::Key::Digit7,
                    KeyCode::Digit8 => quake_input::Key::Digit8,
                    KeyCode::Digit9 => quake_input::Key::Digit9,
                    KeyCode::Equal => quake_input::Key::Equal,
                    KeyCode::KeyA => quake_input::Key::KeyA,
                    KeyCode::KeyB => quake_input::Key::KeyB,
                    KeyCode::KeyC => quake_input::Key::KeyC,
                    KeyCode::KeyD => quake_input::Key::KeyD,
                    KeyCode::KeyE => quake_input::Key::KeyE,
                    KeyCode::KeyF => quake_input::Key::KeyF,
                    KeyCode::KeyG => quake_input::Key::KeyG,
                    KeyCode::KeyH => quake_input::Key::KeyH,
                    KeyCode::KeyI => quake_input::Key::KeyI,
                    KeyCode::KeyJ => quake_input::Key::KeyJ,
                    KeyCode::KeyK => quake_input::Key::KeyK,
                    KeyCode::KeyL => quake_input::Key::KeyL,
                    KeyCode::KeyM => quake_input::Key::KeyM,
                    KeyCode::KeyN => quake_input::Key::KeyN,
                    KeyCode::KeyO => quake_input::Key::KeyO,
                    KeyCode::KeyP => quake_input::Key::KeyP,
                    KeyCode::KeyQ => quake_input::Key::KeyQ,
                    KeyCode::KeyR => quake_input::Key::KeyR,
                    KeyCode::KeyS => quake_input::Key::KeyS,
                    KeyCode::KeyT => quake_input::Key::KeyT,
                    KeyCode::KeyU => quake_input::Key::KeyU,
                    KeyCode::KeyV => quake_input::Key::KeyV,
                    KeyCode::KeyW => quake_input::Key::KeyW,
                    KeyCode::KeyX => quake_input::Key::KeyX,
                    KeyCode::KeyY => quake_input::Key::KeyY,
                    KeyCode::KeyZ => quake_input::Key::KeyZ,
                    KeyCode::Minus => quake_input::Key::Minus,
                    KeyCode::Period => quake_input::Key::Period,
                    KeyCode::Quote => quake_input::Key::Quote,
                    KeyCode::Semicolon => quake_input::Key::Semicolon,
                    KeyCode::Slash => quake_input::Key::Slash,
                    KeyCode::AltLeft => quake_input::Key::AltLeft,
                    KeyCode::AltRight => quake_input::Key::AltRight,
                    KeyCode::Backspace => quake_input::Key::Backspace,
                    KeyCode::ControlLeft => quake_input::Key::ControlLeft,
                    KeyCode::ControlRight => quake_input::Key::ControlRight,
                    KeyCode::Enter => quake_input::Key::Enter,
                    KeyCode::SuperLeft => quake_input::Key::SuperLeft,
                    KeyCode::SuperRight => quake_input::Key::SuperRight,
                    KeyCode::ShiftLeft => quake_input::Key::ShiftLeft,
                    KeyCode::ShiftRight => quake_input::Key::ShiftRight,
                    KeyCode::Space => quake_input::Key::Space,
                    KeyCode::Tab => quake_input::Key::Tab,
                    KeyCode::End => quake_input::Key::End,
                    KeyCode::Home => quake_input::Key::Home,
                    KeyCode::PageDown => quake_input::Key::PageDown,
                    KeyCode::PageUp => quake_input::Key::PageUp,
                    KeyCode::ArrowDown => quake_input::Key::ArrowDown,
                    KeyCode::ArrowLeft => quake_input::Key::ArrowLeft,
                    KeyCode::ArrowRight => quake_input::Key::ArrowRight,
                    KeyCode::ArrowUp => quake_input::Key::ArrowUp,
                    KeyCode::Escape => quake_input::Key::Escape,
                    KeyCode::F1 => quake_input::Key::F1,
                    KeyCode::F2 => quake_input::Key::F2,
                    KeyCode::F3 => quake_input::Key::F3,
                    KeyCode::F4 => quake_input::Key::F4,
                    KeyCode::F5 => quake_input::Key::F5,
                    KeyCode::F6 => quake_input::Key::F6,
                    KeyCode::F7 => quake_input::Key::F7,
                    KeyCode::F8 => quake_input::Key::F8,
                    KeyCode::F9 => quake_input::Key::F9,
                    KeyCode::F10 => quake_input::Key::F10,
                    KeyCode::F11 => quake_input::Key::F11,
                    KeyCode::F12 => quake_input::Key::F12,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => {
                        self.input_manager.on_pressed(Source::Key(key));
                    }
                    ElementState::Released => {
                        self.input_manager.on_released(Source::Key(key));
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let button = match button {
                    MouseButton::Left => quake_input::Button::Left,
                    MouseButton::Right => quake_input::Button::Right,
                    MouseButton::Middle => quake_input::Button::Middle,
                    MouseButton::Back => quake_input::Button::Back,
                    MouseButton::Forward => quake_input::Button::Forward,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => self.input_manager.on_pressed(Source::Button(button)),
                    ElementState::Released => {
                        self.input_manager.on_released(Source::Button(button))
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                window.pre_present_notify();
                render_manager.on_present_frame();
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let ClientPhase::Uninitialized = self.phase {
            return;
        }
        match event {
            DeviceEvent::MouseMotion { delta: (x, y) } => {
                self.input_manager.on_motion(x, y);
            }
            DeviceEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }),
            } => {
                self.input_manager.on_scroll(x, y);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let ClientPhase::Initialized {
            window,
            render_manager,
        } = &mut self.phase
        {
            for command in self
                .input_manager
                .drain()
                .into_iter()
                .filter_map(|intent| ClientCommand::from_str(&intent.0))
            {
                self.event_loop_proxy.send_event(command).unwrap();
            }

            self.world_manager.step().unwrap();

            render_manager.on_acquire_frame().unwrap();
            render_manager.on_draw_frame();
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let ClientPhase::Initialized { render_manager, .. } = &mut self.phase {
            render_manager.destroy();
        }
        self.phase = ClientPhase::Uninitialized;
    }
}
