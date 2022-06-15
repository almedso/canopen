# CANOpen

This is a **Rust** workspace project.
It consists of the crates

* **cot** - CANOpen tool - a cli to inspect and manage a CANOpen application
* **col** - CANOpen library - a library of CANOpen functionality


## Can setup

on linux

```
 $ sudo ip link set can0 type can bitrate 250000
 $ sudo ip link set up can0
 ```

# License

Licensed under MIT license [LICENSE-MIT](LICENSE-MIT).


# Todo

1. dig into sdo frame display/output - better output
2. rework sdo (sdo client -> sdo get rid of processing just datatype presentation)
