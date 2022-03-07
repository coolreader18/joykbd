# joykbd

Translates joy-con -> mouse. Specifically, (by default,) the stick acts like a
thinkpad "nub", ZL/ZR left-click, L/R right-click.

## Usage

Clone this repository and build it using cargo. Alternatively, install it with
`cargo install --git https://github.com/coolreader18/joykbd`.

Connect a Joy-Con to your computer using bluetooth. Ensure you have a driver
for Joy-Cons; I've had no issues with
[`dkms-hid-nintendo`](https://github.com/nicman23/dkms-hid-nintendo)
([aur](https://aur.archlinux.org/packages/hid-nintendo-dkms)).

```sh
joykbd /dev/input/eventNN
# by default, it looks for a device in /dev/input whose name has "Joy-Con" in it
# so, leaving the device path out should be fine in most cases
joykbd
# if the cursor tends to like going to the right more then the left, set
# --adjust-x with a negative value. Vice-versa for leaning left more than right.
joykbd --adjust-x -2000
# if you have bad joycon drift, set --drift-threshold. Axis readings where
# abs(value) < drift-threshold will be ignored. Note that this also makes the
# pointing device less sensitive, unfortunately.
joykbd --drift-threshold 4000
```

## License

This project is licensed under the MIT license. Please see the
[LICENSE](LICENSE) file for more details.
