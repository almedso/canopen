# CANOpen

This is a **Rust** workspace project.
It consists of the crates

* **cot** - CANOpen tool - a cli to inspect and manage a CANOpen application
* **col** - CANOpen library - a library of CANOpen functionality
* **bdd** - CANopen bdd - a Tool to run cucumber specified tests against
            CANopen network

## CAN bus setup

the CANOpen library and therefore also the CANOpen tool and the BDD tool
are based on [sockecan](https://en.wikipedia.org/wiki/SocketCAN).

On a linux system, where a can hardware driver is attached and a
kernel driver is loaded the can bus can be brought up like:

```sh
sudo ip link set can0 type can bitrate 250000
sudo ip link set up can0
```

## Running the CANOpen tool cot.

Like any other rust project you need to have rust installed.

```sh
git clone git@github.com:almedso/canopen.git
cargo run -- --help  # get help information on the CANOpen tool
```

## Running BDD test

see the the [README.md in the bdd crate](./bdd/README.md)
for more details.

## License

Licensed under MIT license [LICENSE-MIT](LICENSE-MIT).

## Todo

Not necessarily in this sequence

* more sdo z.B. multibyte
* unit tests
* refactor remove code duplications

test mermaid


## References

* https://canopennode.github.io/index.html
* https://www.waycon.de/fileadmin/seilzugsensoren/CANopen-Handbuch.pdf
* https://edu.elektronikschule.de/~amann/hambsch/can/CANopen
