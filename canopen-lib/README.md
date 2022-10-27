# CANOpen library

## Restructuring ideas

Frame module

- each frame can be builded
- each frame can be formatted
  - for monitoring)
- each frame can be parsed/converted from CANFrame
- if nessesary a frame gets its submodule like sdo
- each frame can be inspected
  - i.e. payload as enum
  - for filtering

## ToDo

- Document public functions for frames
- Add unit tests for frames
- Add sdo segmented read/write