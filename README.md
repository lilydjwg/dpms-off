Same as `xset dpms force off`, but for Wayland.

It requires [zwlr_output_power_manager_v1](https://wayland.app/protocols/wlr-output-power-management-unstable-v1) and [ext_idle_notifier_v1](https://wayland.app/protocols/ext-idle-notify-v1) support from the Wayland compositer. wlroots supports them.

It turns off all monitors, and then it turns them back on when user activity re-occurs. Don't interrupt the process, or your monitor won't be turned on later!

Install the Rust toolchain and `cargo build --release` to build and find the built binary under `target/release`.
