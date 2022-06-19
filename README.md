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

1. Test SDO updated implementation wod / read
2. test sdo monitor output
3. implement PDO (mapped output)



## References

- https://canopennode.github.io/index.html
- https://www.waycon.de/fileadmin/seilzugsensoren/CANopen-Handbuch.pdf
- https://edu.elektronikschule.de/~amann/hambsch/can/CANopen
