# BDD a canopen Unit

## Subject under test

... is a CANOpen unit that

- is stimulated by some PDO's or SDO OD write requests
- responds by some object dictionary updates
- by some emitted PDO's

## Handling

- The `cucumber` dependency must be set.
- Should be a library project.

Tests need to be added to `Cargo.toml` like

```toml
[[test]]
name = "canopen"
harness = false  # allows Cucumber to print output instead of libtest
```

The rust file need to located at `./tests/canopen.rs`. The name `canonepn` must match.

Within the rust file the feature file to be called are specified.

tests can be run the usual way simply by

```sh
cargo test
```

## Preparation of the test run

Prior to running canopen test the CAN interface must brought into shape.
The CAN Socket API is used.

### On linux

On linux the kernel must support the can adapter hardware.
Run the following two commands:

```sh
sudo ip link set can0 type can bitrate 250000
sudo ip link set up can0
```

to bring the `can0` interface into shape.
Instead of `250000`, pick the bitrate of the can network the SUT runs on.

## Custimization of the test run

```sh
cargo test --test canopen -- --concurrency 1
```

- pick the runner as needed by `--test canopen`
- pass on parameters to the runner by `--`
- canopen needs to run sequencially in order prevent race condiation on can stack
  This is done by `--concurrency 1`
- configure an alternative path glob to select the feature file(s) by `--input <feature glob>`

Figure out more options of the cucumber runner:

```sh
cargo test -- --help
```

## Resources (about rust cucumber)

- [Rust on cucumber general page](https://cucumber.io/docs/installation/rust/)
- [Rust Cucumber Reop](https://github.com/cucumber-rs/cucumber)
- [Crates - Cucumber](https://crates.io/crates/cucumber)
- [Cucumber Doc](https://docs.rs/cucumber/0.13.0/cucumber/)
- [Rust cucumber book](https://cucumber-rs.github.io/cucumber/current/)
