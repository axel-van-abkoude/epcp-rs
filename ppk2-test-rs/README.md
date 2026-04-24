# EPCP: Ppk2 test crate

Extends [ppk2-rs](https://github.com/hdoordt/ppk2-rs) crate by:
- [Setup::flash]: flashes a device and waits for it to finish
- [Setup::measure]: live measurements stored per pin configuration ([Section])

## Examples
Flashing and measuring
```
cargo run --example=flash
```

## Metrics
- [Sections::total_capacity]: total µAh over all sections
- [Sections::total_duration]: total time of all sections
- [Section::total_capacity]: total µAh over one section
- [Section::total_duration]: total time of one section

## WIP
- Adding spans to get std_dev between spans
- macro framework to annotate embedded code
- Adding storage options

