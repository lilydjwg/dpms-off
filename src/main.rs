use wayland_client::{Connection, Dispatch, QueueHandle, Proxy, delegate_noop};
use wayland_client::protocol::{wl_registry, wl_output, wl_seat};
use wayland_protocols::ext::idle_notify::v1::client::{ext_idle_notifier_v1, ext_idle_notification_v1};
use wayland_protocols_wlr::output_power_management::v1::client::{zwlr_output_power_manager_v1, zwlr_output_power_v1::{self, Mode}};

use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
  #[clap(long, value_name="N ms", default_value_t=500, help="milliseconds to wait for activity to stop before turning DPMS off")]
  before: u32,
}

fn main() {
  let cli = Cli::parse();
  run(cli.before);
}

#[derive(Default)]
struct State {
  outputs: Vec<wl_output::WlOutput>,
  seat: Option<wl_seat::WlSeat>,
  output_power_manager: Option<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>,
  idle_notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
  quit: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
  fn event(
    s: &mut Self,
    registry: &wl_registry::WlRegistry,
    event: wl_registry::Event,
    _: &(),
    _: &Connection,
    qh: &QueueHandle<State>,
  ) {
    if let wl_registry::Event::Global { name, interface, .. } = event {
      match &interface[..] {
        "wl_output" => {
          let output = registry.bind::<wl_output::WlOutput, _, _>(name, 2, qh, ());
          s.outputs.push(output);
        }
        "wl_seat" => {
          s.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, 7, qh, ()));
        }
        "zwlr_output_power_manager_v1" => {
          s.output_power_manager =
            Some(registry.bind::<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, _, _>(name, 1, qh, ()));
        }
        "ext_idle_notifier_v1" => {
          s.idle_notifier =
            Some(registry.bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(name, 1, qh, ()));
        }
        _ => {}
      }
    } else if let wl_registry::Event::GlobalRemove { name } = event {
      s.outputs.retain(|o| o.id().protocol_id() != name);
    }
  }
}

delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ignore wl_output::WlOutput);
delegate_noop!(State: ext_idle_notifier_v1::ExtIdleNotifierV1);
delegate_noop!(State: zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1);
delegate_noop!(State: ignore zwlr_output_power_v1::ZwlrOutputPowerV1);

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for State {
  fn event(
    s: &mut Self,
    _: &ext_idle_notification_v1::ExtIdleNotificationV1,
    event: ext_idle_notification_v1::Event,
    _: &(),
    _: &Connection,
    qh: &QueueHandle<Self>,
  ) {
    match event {
      ext_idle_notification_v1::Event::Idled => {
        s.set_mode(Mode::Off, qh);
      },
      ext_idle_notification_v1::Event::Resumed => {
        s.set_mode(Mode::On, qh);
        s.quit = true;
      },
      _ => {}
    }
  }
}

impl State {
  fn set_mode(&self, mode: Mode, qh: &QueueHandle<Self>) {
    let m = self.output_power_manager.as_ref().unwrap();
    for o in &self.outputs {
      let p = m.get_output_power(o, qh, ());
      p.set_mode(mode);
      p.destroy();
    }
  }
}

fn run(before: u32) {
  let conn = Connection::connect_to_env().unwrap();
  let display = conn.display();
  let mut event_queue = conn.new_event_queue();
  let qh = event_queue.handle();
  let _registry = display.get_registry(&qh, ());
  let mut state = State::default();

  event_queue.roundtrip(&mut state).unwrap();

  if state.output_power_manager.is_none() {
    panic!("Error: zwlr_output_power_manager_v1 not supported by the Wayland compositor.");
  }
  if state.idle_notifier.is_none() {
    panic!("Error: ext_idle_notifier_v1 not supported by the Wayland compositor.");
  }

  state.idle_notifier.as_ref().unwrap().get_idle_notification(before, state.seat.as_ref().unwrap(), &qh, ());

  while !state.quit {
    event_queue.blocking_dispatch(&mut state).unwrap();
  }
  event_queue.roundtrip(&mut state).unwrap();
}
