use std::{rc::Rc, cell::{Cell, RefCell}};
use std::time::{Instant, Duration};

use wayland_client::{Display, GlobalManager, Main, global_filter};
use wayland_client::protocol::{wl_output, wl_seat};
use wayland_protocols::wlr::unstable::output_power_management::v1::client::{zwlr_output_power_manager_v1, zwlr_output_power_v1::Mode};

mod proto;

use proto::idle::{org_kde_kwin_idle, org_kde_kwin_idle_timeout};

fn set_mode(m: &Main<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>, o: &Main<wl_output::WlOutput>, mode: Mode) {
  let p = m.get_output_power(o);
  p.set_mode(mode);
  p.destroy();
}

fn main() {
  let display = Display::connect_to_env().unwrap();
  let mut event_queue = display.create_event_queue();
  let attached_display = (*display).clone().attach(event_queue.token());

  let manager: Rc<RefCell<Option<Main<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>>>> = Rc::new(RefCell::new(None));
  let manager2 = manager.clone();
  let outputs = Rc::new(RefCell::new(Vec::new()));
  let outputs2 = outputs.clone();
  let seat = Rc::new(RefCell::new(None));
  let seat2 = seat.clone();
  let idle = Rc::new(RefCell::new(None));
  let idle2 = idle.clone();

  let _globals = GlobalManager::new_with_cb(
    &attached_display,
    global_filter!(
      [wl_output::WlOutput, 2, move |output: Main<wl_output::WlOutput>, _: DispatchData| {
        outputs2.borrow_mut().push(output);
      }],
      [zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, 1, move |m: Main<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>, _: DispatchData| {
        manager2.borrow_mut().replace(m);
      }],
      [wl_seat::WlSeat, 7, move |s: Main<wl_seat::WlSeat>, _: DispatchData| {
        seat2.borrow_mut().replace(s);
      }],
      [org_kde_kwin_idle::OrgKdeKwinIdle, 1, move |idle: Main<org_kde_kwin_idle::OrgKdeKwinIdle>, _: DispatchData| {
        idle2.borrow_mut().replace(idle);
      }]
    )
  );

  event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

  if manager.borrow().is_none() {
    panic!("Error: zwlr_output_power_manager_v1 not supported by the Wayland compositor.");
  }
  if idle.borrow().is_none() {
    panic!("Error: org_kde_kwin_idle not supported by the Wayland compositor.");
  }

  let idle_timeout = idle.borrow().as_ref().unwrap().get_idle_timeout(seat.borrow().as_ref().unwrap(), 500);
  let idle = Rc::new(Cell::new(false));
  let idle2 = idle.clone();
  let resumed = Rc::new(Cell::new(false));
  let resumed2 = resumed.clone();
  idle_timeout.quick_assign(move |_, event, _|
    match event {
      org_kde_kwin_idle_timeout::Event::Idle => {
        idle2.set(true);
      },
      org_kde_kwin_idle_timeout::Event::Resumed => {
        resumed2.set(true);
      },
    }
  );

  let start = Instant::now();
  while !idle.get() {
    event_queue.dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();
  }
  if start.elapsed() > Duration::from_secs(1) {
    // some program inhibits idle for some time, cancel our operation
    return;
  }
  for o in &*outputs.borrow() {
    set_mode(manager.borrow().as_ref().unwrap(), o, Mode::Off);
  }

  while !resumed.get() {
    event_queue.dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();
  }
  for o in &*outputs.borrow() {
    set_mode(manager.borrow().as_ref().unwrap(), o, Mode::On);
  }

  event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
}
