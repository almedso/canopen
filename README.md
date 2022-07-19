# CANOpen

This is a **Rust** workspace project.
It consists of the crates

* **cot** - CANOpen tool - a cli to inspect and manage a CANOpen application
* **col** - CANOpen library - a library of CANOpen functionality
* **bdd** - CANopen bdd - a Tool to run cucumber specified tests against
            CANopen network

--> check for potential README's in the respective contained crates.

## Can setup

on linux

```sh
sudo ip link set can0 type can bitrate 250000
sudo ip link set up can0
```

## License

Licensed under MIT license [LICENSE-MIT](LICENSE-MIT).

## Todo

Not nessecarily in this sequence

* test SDO updated implementation wod / read
* test sdo monitor output
* implement PDO (mapped output)
* improved PDO monitor output
* better input hex, dec, binary
* revise canopen steps in bdd
* unit tests
* refactor remove code duplications

## References

* https://canopennode.github.io/index.html
* https://www.waycon.de/fileadmin/seilzugsensoren/CANopen-Handbuch.pdf
* https://edu.elektronikschule.de/~amann/hambsch/can/CANopen
