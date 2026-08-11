// wlr-foreign-toplevel-management client; runs on its own thread/connection, bridged to GTK via async-channel (events) and calloop (commands).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;

use async_channel::Sender as AsyncSender;
use calloop::channel::{channel, Channel, Sender as CalloopSender};
use calloop_wayland_source::WaylandSource;
use wayland_client::backend::ObjectData;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    self, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    self, ZwlrForeignToplevelManagerV1,
};

pub type ToplevelId = u32;

#[derive(Debug, Clone)]
pub struct ToplevelInfo {
    pub id: ToplevelId,
    pub app_id: String,
    pub title: String,
    pub activated: bool,

    pub maximized: bool,
    pub fullscreen: bool,

    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ToplevelEvent {
    Updated(ToplevelInfo),
    Closed(ToplevelId),
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Activate(ToplevelId),
    Minimize(ToplevelId),
    Close(ToplevelId),
}

#[derive(Default)]
struct PendingToplevel {
    app_id: String,
    title: String,
    activated: bool,
    maximized: bool,
    fullscreen: bool,
    outputs: HashSet<u32>,
}

struct AppState {
    seat: Option<WlSeat>,
    pending: HashMap<ToplevelId, PendingToplevel>,
    handles: HashMap<ToplevelId, ZwlrForeignToplevelHandleV1>,
    events_tx: AsyncSender<ToplevelEvent>,
    outputs: HashMap<u32, String>,
    output_globals: HashMap<u32, WlOutput>,
}

fn handle_id(handle: &ZwlrForeignToplevelHandleV1) -> ToplevelId {
    handle.id().protocol_id()
}

fn output_id(output: &WlOutput) -> u32 {
    output.id().protocol_id()
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version }
                if interface == WlOutput::interface().name =>
            {
                let output: WlOutput = registry.bind(name, version.min(4), qh, ());
                state.output_globals.insert(name, output);
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(output) = state.output_globals.remove(&name) {
                    state.outputs.remove(&output_id(&output));
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            state.outputs.insert(output_id(output), name);
        }
    }
}

impl Dispatch<WlSeat, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _manager: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                let id = handle_id(&toplevel);
                state.handles.insert(id, toplevel);
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {}
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qhandle: &QueueHandle<Self>,
    ) -> Arc<dyn ObjectData> {
        match opcode {
            0 => qhandle.make_data::<ZwlrForeignToplevelHandleV1, ()>(()),
            _ => unreachable!("zwlr_foreign_toplevel_manager_v1 has no other object-creating events"),
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for AppState {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let id = handle_id(handle);
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                state.pending.entry(id).or_default().title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                state.pending.entry(id).or_default().app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                state.pending.entry(id).or_default().outputs.insert(output_id(&output));
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                state.pending.entry(id).or_default().outputs.remove(&output_id(&output));
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: raw } => {
                // wire values: 0 maximized, 1 minimized, 2 activated, 3 fullscreen
                let mut has = [false; 4];
                for chunk in raw.chunks_exact(4) {
                    let value = u32::from_ne_bytes(chunk.try_into().unwrap());
                    if let Some(slot) = has.get_mut(value as usize) {
                        *slot = true;
                    }
                }
                let pending = state.pending.entry(id).or_default();
                pending.maximized = has[0];
                pending.activated = has[2];
                pending.fullscreen = has[3];
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                if let Some(p) = state.pending.get(&id) {
                    let outputs = p
                        .outputs
                        .iter()
                        .filter_map(|oid| state.outputs.get(oid).cloned())
                        .collect();
                    let info = ToplevelInfo {
                        id,
                        app_id: p.app_id.clone(),
                        title: p.title.clone(),
                        activated: p.activated,
                        maximized: p.maximized,
                        fullscreen: p.fullscreen,
                        outputs,
                    };
                    let _ = state.events_tx.try_send(ToplevelEvent::Updated(info));
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.pending.remove(&id);
                if let Some(h) = state.handles.remove(&id) {
                    h.destroy();
                }
                let _ = state.events_tx.try_send(ToplevelEvent::Closed(id));
            }
            _ => {}
        }
    }
}

pub fn spawn(events_tx: AsyncSender<ToplevelEvent>) -> CalloopSender<Command> {
    let (cmd_tx, cmd_rx) = channel::<Command>();

    thread::Builder::new()
        .name("wlr-toplevel".into())
        .spawn(move || {
            if let Err(err) = run(events_tx, cmd_rx) {
                eprintln!(
                    "jdock: wlr-foreign-toplevel-management unavailable ({err}); \
                     running apps won't show in the dock, only pinned ones"
                );
            }
        })
        .expect("failed to spawn wlr-toplevel thread");

    cmd_tx
}

fn run(
    events_tx: AsyncSender<ToplevelEvent>,
    cmd_rx: Channel<Command>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, queue) = registry_queue_init::<AppState>(&conn)?;
    let qh = queue.handle();

    let manager: ZwlrForeignToplevelManagerV1 = globals
        .bind(&qh, 1..=3, ())
        .map_err(|e| format!("compositor does not implement wlr-foreign-toplevel-management: {e}"))?;
    let seat: Option<WlSeat> = globals.bind(&qh, 1..=9, ()).ok();

    let output_names: Vec<(u32, u32)> = globals.contents().with_list(|list| {
        list.iter()
            .filter(|g| g.interface == WlOutput::interface().name)
            .map(|g| (g.name, g.version.min(4)))
            .collect()
    });
    let mut output_globals = HashMap::new();
    for (name, version) in output_names {
        let output: WlOutput = globals.registry().bind(name, version, &qh, ());
        output_globals.insert(name, output);
    }

    let mut state = AppState {
        seat,
        pending: HashMap::new(),
        handles: HashMap::new(),
        events_tx,
        outputs: HashMap::new(),
        output_globals,
    };

    let mut event_loop: calloop::EventLoop<AppState> = calloop::EventLoop::try_new()?;
    let handle = event_loop.handle();

    WaylandSource::new(conn, queue)
        .insert(handle.clone())
        .map_err(|e| format!("failed to register wayland event source: {e}"))?;

    handle
        .insert_source(cmd_rx, move |event, _, state: &mut AppState| {
            let calloop::channel::Event::Msg(cmd) = event else {
                return;
            };
            match cmd {
                Command::Activate(id) => {
                    if let (Some(handle), Some(seat)) = (state.handles.get(&id), state.seat.as_ref()) {
                        handle.activate(seat);
                    }
                }
                Command::Minimize(id) => {
                    if let Some(handle) = state.handles.get(&id) {
                        handle.set_minimized();
                    }
                }
                Command::Close(id) => {
                    if let Some(handle) = state.handles.get(&id) {
                        handle.close();
                    }
                }
            }
        })
        .map_err(|e| format!("failed to register command source: {e}"))?;

    let _manager = manager;
    loop {
        event_loop.dispatch(None, &mut state)?;
    }
}
