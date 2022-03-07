use clap::Parser;
use evdev::uinput::VirtualDeviceBuilder;
use evdev::{
    AbsoluteAxisType, AttributeSet, Device, EventType, InputEvent, InputEventKind, Key,
    RelativeAxisType,
};
use std::path::PathBuf;
use tokio::time;

#[derive(Parser)]
struct Args {
    /// The path to the evdev device file representing the joy-con you want to use. By default,
    /// joykbd searches for the first device that has "Joy-Con" in it's name.
    device: Option<PathBuf>,
    /// The cursor speed; how fast it'll move when the stick is held all the way to one direction.
    #[clap(long, default_value_t = 20.0)]
    speed: f64,
    /// The repeat timeout for the pseudo-mouse, in milliseconds
    #[clap(long, default_value_t = 16)]
    repeat_timeout: u64,
    #[clap(long, default_value_t = 2000)]
    drift_threshold: u32,
    #[clap(long, allow_hyphen_values = true, default_value_t = 0)]
    adjust_x: i32,
    #[clap(long, allow_hyphen_values = true, default_value_t = 0)]
    adjust_y: i32,
}

impl Args {
    fn stick_constants(&self) -> StickConstants {
        StickConstants {
            factor: self.speed / 30_000f64.powi(5),
            drift_threshold: self.drift_threshold,
            adjustments: (self.adjust_x, self.adjust_y),
        }
    }
}

struct StickConstants {
    factor: f64,
    drift_threshold: u32,
    adjustments: (i32, i32),
}

enum Axis {
    X,
    Y,
}

impl StickConstants {
    fn map_axis(&self, axis: Axis, value: i32) -> i32 {
        let value = value
            + match axis {
                Axis::X => self.adjustments.0,
                Axis::Y => self.adjustments.1,
            };
        if value.unsigned_abs() < self.drift_threshold {
            0
        } else {
            (f64::from(value).powi(5) * self.factor) as i32
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let stick_constants = args.stick_constants();
    let repeat_timeout = time::Duration::from_millis(args.repeat_timeout);

    let dev = if let Some(dev_path) = &args.device {
        Device::open(dev_path)?
    } else {
        eprintln!("Searching for joy-con, please wait...");
        let (_, dev) = evdev::enumerate()
            .find(|(_, dev)| dev.name().map_or(false, |name| name.contains("Joy-Con")))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find a connected joy-con, please pass one on the command line"
                )
            })?;
        eprintln!("Found joy-con!");
        dev
    };

    let mut uinp = VirtualDeviceBuilder::new()?
        .name("joykbd")
        .with_relative_axes(&AttributeSet::from_iter([
            RelativeAxisType::REL_X,
            RelativeAxisType::REL_Y,
        ]))?
        .with_keys(&AttributeSet::from_iter([
            Key::BTN_LEFT,
            Key::BTN_RIGHT,
            Key::BTN_MIDDLE,
            Key::KEY_UP,
            Key::KEY_RIGHT,
            Key::KEY_DOWN,
            Key::KEY_LEFT,
        ]))?
        .build()?;

    let mut ev_stream = dev.into_event_stream()?;

    let sleep_x = time::sleep(time::Duration::MAX);
    let mut prev_x = 0;
    let sleep_y = time::sleep(time::Duration::MAX);
    let mut prev_y = 0;
    tokio::pin!(sleep_x, sleep_y);

    loop {
        tokio::select! {
            ev = ev_stream.next_event() => {
                let ev = if let Some(ev) = map_event(ev?, &stick_constants) {
                    ev
                } else {
                    continue
                };
                match ev.kind() {
                    InputEventKind::RelAxis(RelativeAxisType::REL_X) => {
                        sleep_x.as_mut().reset(time::Instant::now() + repeat_timeout);
                        prev_x = ev.value();
                    }
                    InputEventKind::RelAxis(RelativeAxisType::REL_Y) => {
                        sleep_y.as_mut().reset(time::Instant::now() + repeat_timeout);
                        prev_y = ev.value();
                    }
                    _ => {}
                }
                uinp.emit(&[ev])?;
            }
            () = &mut sleep_x => {
                uinp.emit(&[InputEvent::new(
                    EventType::RELATIVE,
                    RelativeAxisType::REL_X.0,
                    prev_x,
                )])?;
                sleep_x.as_mut().reset(time::Instant::now() + repeat_timeout);
            }
            () = &mut sleep_y => {
                uinp.emit(&[InputEvent::new(
                    EventType::RELATIVE,
                    RelativeAxisType::REL_Y.0,
                    prev_y,
                )])?;
                sleep_y.as_mut().reset(time::Instant::now() + repeat_timeout);
            }
        }
    }
}

fn map_event(ev: InputEvent, stick_constants: &StickConstants) -> Option<InputEvent> {
    match ev.kind() {
        // ZL/ZR
        InputEventKind::Key(Key::BTN_TR2) | InputEventKind::Key(Key::BTN_TL2) => Some(
            InputEvent::new(EventType::KEY, Key::BTN_LEFT.code(), ev.value()),
        ),
        // L
        InputEventKind::Key(Key::BTN_TR) | InputEventKind::Key(Key::BTN_TL) => Some(
            InputEvent::new(EventType::KEY, Key::BTN_RIGHT.code(), ev.value()),
        ),
        // press R stick
        InputEventKind::Key(Key::BTN_THUMBR) | InputEventKind::Key(Key::BTN_THUMBL) => Some(
            InputEvent::new(EventType::KEY, Key::BTN_MIDDLE.code(), ev.value()),
        ),
        // A
        InputEventKind::Key(Key::BTN_EAST) => Some(InputEvent::new(
            EventType::KEY,
            Key::KEY_RIGHT.code(),
            ev.value(),
        )),
        // B
        InputEventKind::Key(Key::BTN_SOUTH) => Some(InputEvent::new(
            EventType::KEY,
            Key::KEY_DOWN.code(),
            ev.value(),
        )),
        // X
        InputEventKind::Key(Key::BTN_NORTH) => Some(InputEvent::new(
            EventType::KEY,
            Key::KEY_UP.code(),
            ev.value(),
        )),
        // Y
        InputEventKind::Key(Key::BTN_WEST) => Some(InputEvent::new(
            EventType::KEY,
            Key::KEY_LEFT.code(),
            ev.value(),
        )),
        InputEventKind::AbsAxis(AbsoluteAxisType::ABS_RX)
        | InputEventKind::AbsAxis(AbsoluteAxisType::ABS_X) => Some(InputEvent::new(
            EventType::RELATIVE,
            RelativeAxisType::REL_X.0,
            stick_constants.map_axis(Axis::X, ev.value()),
        )),
        InputEventKind::AbsAxis(AbsoluteAxisType::ABS_RY)
        | InputEventKind::AbsAxis(AbsoluteAxisType::ABS_Y) => Some(InputEvent::new(
            EventType::RELATIVE,
            RelativeAxisType::REL_Y.0,
            stick_constants.map_axis(Axis::Y, ev.value()),
        )),
        _ => None,
    }
}
