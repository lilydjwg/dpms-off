use std::{rc::Rc, cell::{Cell, RefCell}};

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

  for o in &*outputs.borrow() {
    set_mode(manager.borrow().as_ref().unwrap(), o, Mode::Off);
  }
  let idle_timeout = idle.borrow().as_ref().unwrap().get_idle_timeout(seat.borrow().as_ref().unwrap(), 1000);
  let outputs3 = outputs.clone();
  let manager3 = manager.clone();
  let resumed = Rc::new(Cell::new(false));
  let resumed2 = resumed.clone();
  idle_timeout.quick_assign(move |_, event, _| {
    if let org_kde_kwin_idle_timeout::Event::Resumed = event {
      for o in &*outputs3.borrow() {
        set_mode(manager3.borrow().as_ref().unwrap(), o, Mode::On);
      }
      resumed2.set(true);
    }
  });

  while !resumed.get() {
    event_queue.dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ }).unwrap();
  }
  event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
}
